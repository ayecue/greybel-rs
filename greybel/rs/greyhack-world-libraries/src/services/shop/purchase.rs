//! Buying an exploit into a computer's files (`BankHelperServer.BuyItemServerRpc`).

use greyhack_behaviour::{
    BYTES_PER_MB, FileData, LibraryKind, MAX_FILES, MAX_FILES_PER_FOLDER, Permission,
    PermissionSet, coupon_cost, exploit_file_name, is_valid_version, path,
};
use greyhack_world_core::{ComputerId, FileFailure, Node, Result, User};
use greyhack_world_economy::{BankTransaction, PurchaseFailure};

use super::failure::ExploitShopFailure;
use crate::{Exploit, Library, NodeDraft};

/// Exploit of a library version to buy and where it goes.
#[derive(Debug, Clone, Copy)]
pub struct ExploitPurchase<'a> {
    pub library: LibraryKind,
    pub version: &'a str,
    /// The exploit's name in the shop.
    pub exploit: &'a str,
    /// Folder of the destination computer the file is created in.
    pub folder: &'a str,
    /// Whether the file is a compiled program (`Downloads.compileScript`) rather than a source file named
    /// `<exploit>.src`.
    pub compile: bool,
}

/// Methods of [`World`] for exploits shop purchase.
impl Shop {
    pub async fn buy_exploit(
        &self,
        target: ComputerId,
        user: &str,
        terminal_computers: &[ComputerId],
        purchase: &ExploitPurchase<'_>,
    ) -> Result<std::result::Result<String, ExploitShopFailure>> {
        let file_name = exploit_file_name(purchase.exploit, purchase.compile);

        // `BankHelperServer.BuyItem`
        if !path::is_valid(purchase.folder) {
            return Ok(Err(ExploitShopFailure::InvalidPath));
        }

        if self.networks.topology.computer(target).await?.is_none() {
            return Ok(Err(ExploitShopFailure::UnknownComputer));
        }

        if !self
            .networks
            .reach
            .is_network_active(target, terminal_computers)
            .await?
        {
            return Ok(Err(ExploitShopFailure::NoInternet));
        }

        let account = self.computers.users.user(target, user).await?;
        let Some(folder) = self
            .last_folder(target, purchase.folder, account.as_ref())
            .await?
        else {
            return Ok(Err(ExploitShopFailure::MissingFolder(
                purchase.folder.to_owned(),
            )));
        };

        if let Some(real) = &folder
            && !self
                .files
                .permissions
                .can_modify_contents(real, account.as_ref())
                .await?
        {
            return Ok(Err(ExploitShopFailure::NoCreatePermission {
                folder: purchase.folder.to_owned(),
                name: file_name,
            }));
        }

        // `BankHelperServer.DownloadExploit`
        let Some((library, exploit)) = self.find_exploit(purchase).await? else {
            return Ok(Err(ExploitShopFailure::UnknownExploit));
        };
        let (Some(source), Some(price)) = (exploit.source(&library), exploit.price(&library))
        else {
            return Ok(Err(ExploitShopFailure::UnknownExploit));
        };

        if let Err(failure) = self.pay_shop_item(price).await? {
            return Ok(Err(failure));
        }

        // `DownloadsHelperServer.DownloadFileEnd`, `PlayerUtils.CrearArchivo`: the player has paid when the file is
        // made, so a failure retains the payment.
        let data = if purchase.compile {
            FileData::binary_from_source(&source)
        } else {
            FileData::text(&source)
        };

        let Some(folder) = folder else {
            return Ok(Err(ExploitShopFailure::File(FileFailure::MissingFolder(
                purchase.folder.to_owned(),
            ))));
        };

        if let Err(failure) = self
            .create_bought_file(&folder, user, &file_name, data)
            .await?
        {
            return Ok(Err(ExploitShopFailure::File(failure)));
        }

        Ok(Ok(path::join(&folder.path, &file_name)))
    }

    pub async fn coupons(&self, computer: ComputerId) -> Result<u32> {
        self.exploits.coupons(computer).await
    }

    pub async fn set_coupons(&self, computer: ComputerId, coupons: u32) -> Result<()> {
        self.exploits.set_coupons(computer, coupons).await
    }

    pub async fn pay_purchase(
        &self,
        price: i64,
        reason: &str,
        coupons: bool,
    ) -> Result<std::result::Result<(), PurchaseFailure>> {
        let Some(number) = self.bank.player_bank().await? else {
            return Ok(Err(PurchaseFailure::NoBankAccount));
        };
        let Some(mut bank) = self.bank.bank(&number).await? else {
            return Ok(Err(PurchaseFailure::UnknownBankAccount(number)));
        };
        let mut price = price;
        let coupon_count = i64::from(self.exploits.coupons(self.world.player()).await?);

        if price > 0 && coupons && coupon_count > 0 {
            let cost = coupon_cost(price);

            if coupon_count >= cost {
                price = 0;
                self.exploits
                    .set_coupons(
                        self.world.player(),
                        u32::try_from(coupon_count - cost).expect("coupons are not negative"),
                    )
                    .await?;
            }
        }

        if bank.money < price as f64 {
            return Ok(Err(PurchaseFailure::InsufficientFunds));
        }

        bank.record_transaction(BankTransaction::payment(
            price,
            reason,
            &self.world.date(),
            true,
        ));
        self.bank.save_bank(&bank).await?;
        self.bank.save_bank_transactions(&bank).await?;

        Ok(Ok(()))
    }
}

impl Shop {
    pub async fn find_exploit(
        &self,
        purchase: &ExploitPurchase<'_>,
    ) -> Result<Option<(Library, Exploit)>> {
        if !is_valid_version(purchase.version) {
            return Ok(None);
        }

        let Some((id, library)) = self
            .shop_library(purchase.library, purchase.version)
            .await?
        else {
            return Ok(None);
        };
        let exploits = self.library_exploits(id, &library).await?;

        Ok(exploits
            .into_iter()
            .find(|exploit| exploit.name == purchase.exploit)
            .map(|exploit| (library, exploit)))
    }

    pub async fn last_folder(
        &self,
        computer: ComputerId,
        folder_path: &str,
        account: Option<&User>,
    ) -> Result<Option<Option<Node>>> {
        let Some(mut folder) = self.files.lookup.root(computer).await? else {
            return Ok(None);
        };

        if folder_path == "/" {
            return Ok(Some(Some(folder)));
        }

        let mut exists = true;

        for (_, full) in path::steps(folder_path) {
            if exists
                && !self
                    .files
                    .permissions
                    .has_node_permission(&folder, Permission::Execute, account)
                    .await?
            {
                return Ok(None);
            }

            if !exists {
                continue;
            }

            match self.files.lookup.folder(computer, &full, false).await? {
                Some(subfolder) => folder = subfolder,
                None => {
                    if !self
                        .files
                        .permissions
                        .can_modify_contents(&folder, account)
                        .await?
                    {
                        return Ok(None);
                    }

                    exists = false;
                }
            }
        }

        Ok(Some(exists.then_some(folder)))
    }

    pub async fn pay_shop_item(
        &self,
        price: i64,
    ) -> Result<std::result::Result<(), ExploitShopFailure>> {
        if price <= 0 {
            return Ok(Ok(()));
        }

        Ok(self
            .pay_purchase(price, "Shop item purchased", true)
            .await?
            .map_err(ExploitShopFailure::from))
    }

    pub async fn create_bought_file(
        &self,
        folder: &Node,
        user: &str,
        name: &str,
        data: FileData,
    ) -> Result<std::result::Result<(), FileFailure>> {
        if self.files.sizes.file_count(folder.computer).await? > MAX_FILES {
            return Ok(Err(FileFailure::TooManyFiles));
        }

        if data.size as f64 / BYTES_PER_MB
            > self
                .files
                .sizes
                .available_storage_mb(folder.computer)
                .await?
        {
            return Ok(Err(FileFailure::DiskFull));
        }

        if self.files.lookup.children(folder).await?.len() > MAX_FILES_PER_FOLDER {
            return Ok(Err(FileFailure::FolderFull));
        }

        self.generator
            .add_full_node(
                folder,
                &NodeDraft::file(name, user, PermissionSet::default_for(false), data),
            )
            .await?;

        Ok(Ok(()))
    }
}

/// A world service with only its declared dependencies.
pub struct Shop {
    pub(crate) world: greyhack_world_core::WorldRef,
    pub(crate) exploits: std::rc::Rc<crate::storage::ExploitRepository>,
    pub(crate) libraries: std::rc::Rc<crate::storage::LibraryRepository>,
    pub(crate) computers: greyhack_world_modules::Dep<greyhack_world_core::ComputerService>,
    pub(crate) files: greyhack_world_modules::Dep<greyhack_world_core::FileSystemService>,
    pub(crate) networks: greyhack_world_modules::Dep<greyhack_world_core::NetworkService>,
    pub(crate) bank: greyhack_world_modules::Dep<greyhack_world_economy::Bank>,
    pub(crate) generator: greyhack_world_modules::Dep<crate::Generator>,
}

impl Shop {
    pub(crate) fn new(
        registry: &std::rc::Rc<greyhack_world_modules::Registry>,
        exploits: std::rc::Rc<crate::storage::ExploitRepository>,
        libraries: std::rc::Rc<crate::storage::LibraryRepository>,
    ) -> Self {
        Self {
            world: registry.world::<greyhack_world_core::WorldRef>().clone(),
            exploits,
            libraries,
            computers: registry.get(),
            files: registry.get(),
            networks: registry.get(),
            bank: registry.get(),
            generator: registry.get(),
        }
    }
}
