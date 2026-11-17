//! Debug authentication, library inspection, patching, tests, and zero-day payloads.

use greyhack_behaviour::{
    CompanyAlign, FileKind, LIBRARY_FOLDER, MAX_DEBUG_LOGIN_ATTEMPTS, ROOT_USER, Word,
    error_lines_match, fixed_payload_user, format_auto_patch_date, is_rumor_time, path,
};
use greyhack_world_assets::GameTextTable;
use greyhack_world_core::{ComputerId, NodeId, Result};
use greyhack_world_economy::{BankTransaction, PurchaseFailure};

use crate::generation::challenge::normalized_code;
use crate::model::zero_day_rumor::{
    CREDENTIALS_IN_USE, DebugCredentials, DebugOffer, DebugPurchase, INCORRECT_OFFER_CREDENTIALS,
    NpcDebugCredentials, OFFER_NOT_AVAILABLE, ONE_OFFER_PER_PLAYER, SALE_CLOSED,
    USED_OFFER_CREDENTIALS, ZeroDayRumor,
};
use crate::{DebugChallenge, DebugLibraryState, ZeroDay, ZeroDayObjectKind};

/// Patch status returned by debug-library inspection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DebugLibraryPatch {
    /// The library is missing or unpatched at its current seed.
    NoPatch,
    /// The library is patched at its current seed.
    PatchPresent,
    /// Scheduled automatic patch date.
    AutoPatchAt(String),
}

/// Result of a debug-library zero-day payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DebugPayload {
    /// Payload execution error.
    Failure(DebugFailure),
    /// Partial objects returned by the payload.
    Objects(Vec<PartialObject>),
}

/// Debug-library errors (`MetaLibIntrinsics.debug_tools`, `DebugLibraryIntrinsics.payload`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DebugFailure {
    /// The password is invalid or the NPC credentials do not exist.
    IncorrectPassword,
    /// The password belongs to a different player.
    CredentialsInUse,
    /// The account exceeded its login-failure limit.
    LoginLockout,
    /// No library file exists at the path.
    MissingFile,
    /// The library is outside `/lib`.
    OutsideLib,
    /// The library is patched against this zero-day.
    PatchPresent,
    /// The memory zone contains no zero-day.
    NoVulnerability,
    /// The supplied payload file cannot be accessed.
    UnreachableResource(String),
    /// Inspection found that the current challenge was already patched.
    ScanPatchPresent,
    /// Testing found that the current challenge was already patched.
    TestPatchPresent,
    /// The patch file is missing or binary.
    InvalidPatchFile,
    /// The submitted patch differs from the generated correct code.
    IncorrectPatch,
    /// This challenge is already patched.
    PatchAlreadyPresent,
    /// An earlier incorrect unit-test response blocked the debug account.
    UnitTestLockout,
    /// Submitted error lines do not exactly match challenge error lines.
    IncorrectErrorLines,
}

/// Zero-day result object limited to `methods`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PartialObject {
    /// Partial computer access using `user` privileges.
    Computer {
        /// Computer accessed by the object.
        computer: ComputerId,
        /// User privileges exposed by the object.
        user: String,
        /// Exposed object methods.
        methods: Vec<String>,
    },
    /// Partial file access as `user`.
    File {
        /// Computer storing the file.
        computer: ComputerId,
        /// File returned by the payload.
        node: NodeId,
        /// File acquisition path.
        path: String,
        /// User privileges exposed by the object.
        user: String,
        /// Exposed object methods.
        methods: Vec<String>,
    },
    /// Partial library-file access as `user`.
    Library {
        /// Computer storing the library.
        computer: ComputerId,
        /// Library acquisition path.
        path: String,
        /// User privileges exposed by the object.
        user: String,
        /// Exposed object methods.
        methods: Vec<String>,
    },
}

/// Methods of [`World`] for exploits debug.
impl Debug {
    /// Reads the saved zero-day system and all per-player request state.
    pub async fn rumor(&self) -> Result<ZeroDayRumor> {
        self.rows.zero_day_state().await
    }

    /// Restores mirrored game state without manufacturing credentials through an administrator action.
    pub async fn restore_rumor(&self, rumor: &ZeroDayRumor) -> Result<()> {
        self.rows.save_zero_day_state(rumor).await
    }

    pub async fn rumor_ip(&self) -> Result<String> {
        Ok(self.rumor().await?.ip_address)
    }

    /// Stores the Neurobox address after its network has been fully generated and configured.
    pub async fn set_rumor_ip(&self, public_ip: &str) -> Result<()> {
        let mut rumor = self.rumor().await?;
        rumor.ip_address = public_ip.to_owned();
        self.restore_rumor(&rumor).await
    }

    /// Clears exactly the state `IsAvailableRumor` clears when the rumor window closes.
    pub async fn clear_rumor(&self) -> Result<()> {
        let mut rumor = self.rumor().await?;
        rumor.ip_address.clear();
        rumor.requested_credentials.clear();
        rumor.npc_passwords.clear();
        rumor.offers.clear();
        self.restore_rumor(&rumor).await
    }

    /// Applies the strict, per-player thirty-day request rule. Network lifecycle belongs to creation.
    pub async fn is_available_rumor(&self, player_id: &str, sync_db: bool) -> Result<bool> {
        if !is_rumor_time(self.world.time_seconds()) {
            return Ok(false);
        }

        let now = self.world.time_seconds();
        let mut rumor = self.rumor().await?;
        let player = rumor.players.entry(player_id.to_owned()).or_default();
        let available = player
            .requested_at
            .is_none_or(|requested| now.saturating_sub(requested) > 30 * 86_400);

        if available && sync_db {
            player.requested_at = Some(now);
            player.solo_attempts = 0;
            player.solo_mail_received = false;
            player.network_bindings.clear();
            player.failed_unit_testing = false;
            player.login_attempts = 0;
            self.restore_rumor(&rumor).await?;
        }

        Ok(available)
    }

    /// Returns the same credentials on every request by one player, and a unique surname-plus-number user otherwise.
    pub async fn get_npc_password(&self, player_id: &str) -> Result<DebugCredentials> {
        let mut rumor = self.rumor().await?;
        if let Some(credentials) = rumor.requested_credentials.get(player_id) {
            return Ok(credentials.clone());
        }

        let mut random = self.world.world_mut().rng.fork("zero-day-npc-password");
        let password = Word::Password.generate(&mut random);
        let user = loop {
            let candidate = format!(
                "{}{}",
                Word::Surname.generate(&mut random),
                random.range(0, 99_999)
            );
            if !rumor.npc_passwords.contains_key(&candidate) {
                break candidate;
            }
        };
        let credentials = DebugCredentials {
            user: user.clone(),
            password: password.clone(),
        };
        rumor.npc_passwords.insert(
            user,
            NpcDebugCredentials {
                password,
                player_id: None,
            },
        );
        rumor
            .requested_credentials
            .insert(player_id.to_owned(), credentials.clone());
        self.restore_rumor(&rumor).await?;

        Ok(credentials)
    }

    /// Answers a good/bad zero-day social-engineering mail as `Persona.ProcesaCredsDebug` does.
    pub async fn answer_zero_day(
        &self,
        public_ip: &str,
        user: &str,
        from_mail: &str,
        answer: CompanyAlign,
        player_id: &str,
    ) -> Result<Option<String>> {
        let Some(router) = self.networks.routers.router_at(public_ip).await? else {
            return Ok(None);
        };
        let Some(npc) = self
            .npcs
            .npcs_in(router.network)
            .await?
            .into_iter()
            .find(|npc| npc.user == user)
        else {
            return Ok(None);
        };
        if npc.company_align == CompanyAlign::None {
            return Ok(Some("I don't know what you're talking about".to_owned()));
        }
        let Some(address) = self.npcs_mail.mail_address(npc.person).await? else {
            return Ok(None);
        };
        if from_mail == address {
            return Ok(Some(
                "Wait... is this message from my account? Who are you and how did you get in??"
                    .to_owned(),
            ));
        }
        if npc.day_zero_contact.as_deref() != Some(from_mail) {
            return Ok(Some(
                "Who are you? I don't recognize that email address and I don't plan to give you anything."
                    .to_owned(),
            ));
        }
        if answer != CompanyAlign::None && answer != npc.company_align {
            return Ok(Some(game_text("ANSWER_ZERODAY_WRONG")));
        }

        let credentials = self.get_npc_password(player_id).await?;
        Ok(Some(
            game_text("ANSWER_ZERODAY_OK")
                .replace("$USER", &credentials.user)
                .replace("$PASSWORD", &credentials.password),
        ))
    }

    /// Validates and, on the first successful login, binds NPC credentials to `player_id`.
    pub async fn login_debug_for_player(
        &self,
        player_id: &str,
        user: &str,
        password: &str,
    ) -> Result<Result<(), DebugFailure>> {
        let mut rumor = self.rumor().await?;
        if rumor
            .players
            .get(player_id)
            .is_some_and(|player| player.login_attempts >= MAX_DEBUG_LOGIN_ATTEMPTS)
        {
            return Ok(Err(DebugFailure::LoginLockout));
        }

        let valid = rumor
            .npc_passwords
            .get(user)
            .is_some_and(|credentials| credentials.password == password);
        if !valid {
            rumor
                .players
                .entry(player_id.to_owned())
                .or_default()
                .login_attempts += 1;
            self.restore_rumor(&rumor).await?;
            return Ok(Err(DebugFailure::IncorrectPassword));
        }

        let credentials = rumor
            .npc_passwords
            .get_mut(user)
            .expect("valid credentials exist");
        if credentials
            .player_id
            .as_deref()
            .is_some_and(|owner| owner != player_id)
        {
            return Ok(Err(DebugFailure::CredentialsInUse));
        }
        credentials.player_id = Some(player_id.to_owned());
        self.restore_rumor(&rumor).await?;

        Ok(Ok(()))
    }

    pub async fn debug_library(
        &self,
        computer: ComputerId,
        path: &str,
    ) -> Result<Option<DebugLibraryState>> {
        let Some(node) = self.files.lookup.file(computer, path, false).await? else {
            return Ok(None);
        };

        self.rows
            .debug_library(
                computer,
                node.id,
                &self.world.world_seed(),
                self.world.time_seconds(),
            )
            .await
    }

    /// Inserts the seed-derived zero-day into a saved library, as `Library.GenerateZeroDayVulnerability` does.
    pub async fn unlock_debug_library(&self, computer: ComputerId, path: &str) -> Result<bool> {
        let Some(node) = self.files.lookup.file(computer, path, false).await? else {
            return Ok(false);
        };
        let Some(state) = self.debug_library(computer, path).await? else {
            return Ok(false);
        };
        let Some(mut library) = self
            .generator
            .full_file_library(computer, path, false)
            .await?
        else {
            return Ok(false);
        };
        let zero_day = state.zero_day();
        let zone = crate::MemoryZone {
            address: zero_day.address.clone(),
            hide: false,
            weaknesses: vec![zero_day.vulnerability],
        };
        if let Some(existing) = library
            .zones
            .iter_mut()
            .find(|zone| zone.address == zero_day.address)
        {
            *existing = zone;
        } else {
            library.zones.push(zone);
        }
        let Some(id) = self.files.node_library(node.id).await? else {
            return Ok(false);
        };
        self.generator
            .libraries
            .replace_library_zones(id, &library)
            .await?;

        Ok(true)
    }

    pub async fn save_debug_library(&self, library: &DebugLibraryState) -> Result<()> {
        self.rows.set_debug_library(library).await
    }

    pub async fn debug_library_patch(
        &self,
        computer: ComputerId,
        path: &str,
        date: bool,
    ) -> Result<DebugLibraryPatch> {
        let Some(library) = self.debug_library(computer, path).await? else {
            return Ok(DebugLibraryPatch::NoPatch);
        };

        if date {
            return Ok(library
                .auto_patch_at
                .map_or(DebugLibraryPatch::NoPatch, |at| {
                    DebugLibraryPatch::AutoPatchAt(format_auto_patch_date(at))
                }));
        }

        Ok(if library.is_patched(self.world.time_seconds()) {
            DebugLibraryPatch::PatchPresent
        } else {
            DebugLibraryPatch::NoPatch
        })
    }

    pub async fn scan_debug_library(
        &self,
        computer: ComputerId,
        path: &str,
    ) -> Result<Result<DebugChallenge, DebugFailure>> {
        let Some(library) = self.debug_library(computer, path).await? else {
            return Ok(Err(DebugFailure::MissingFile));
        };

        if library.is_patched(self.world.time_seconds()) {
            return Ok(Err(DebugFailure::ScanPatchPresent));
        }

        Ok(Ok(library.challenge()))
    }

    pub async fn patch_debug_library(
        &self,
        computer: ComputerId,
        path: &str,
        patch_path: &str,
    ) -> Result<Result<(), DebugFailure>> {
        let Some(mut library) = self.debug_library(computer, path).await? else {
            return Ok(Err(DebugFailure::MissingFile));
        };
        let content = self
            .files
            .lookup
            .file(computer, patch_path, true)
            .await?
            .and_then(|node| node.data().cloned());
        let Some(content) = content else {
            return Ok(Err(DebugFailure::InvalidPatchFile));
        };

        if content.is_binary() {
            return Ok(Err(DebugFailure::InvalidPatchFile));
        }

        // `CodeComparer.IsCodeCorrect` ignores all whitespace and letter case.
        if normalized_code(&content.content) != normalized_code(&library.challenge().correct_code) {
            return Ok(Err(DebugFailure::IncorrectPatch));
        }

        if library.is_patched(self.world.time_seconds()) {
            return Ok(Err(DebugFailure::PatchAlreadyPresent));
        }

        library.add_patch(self.world.time_seconds());
        self.save_debug_library(&library).await?;

        Ok(Ok(()))
    }

    pub async fn run_debug_library_tests(
        &self,
        computer: ComputerId,
        path: &str,
        player_id: &str,
        error_lines: &[i64],
    ) -> Result<Result<ZeroDay, DebugFailure>> {
        if self
            .rumor()
            .await?
            .players
            .get(player_id)
            .is_some_and(|player| player.failed_unit_testing)
        {
            return Ok(Err(DebugFailure::UnitTestLockout));
        }

        let Some(library) = self.debug_library(computer, path).await? else {
            return Ok(Err(DebugFailure::MissingFile));
        };

        if library.is_patched(self.world.time_seconds()) {
            return Ok(Err(DebugFailure::TestPatchPresent));
        }

        if !error_lines_match(error_lines, &library.challenge().error_lines) {
            let mut rumor = self.rumor().await?;
            rumor
                .players
                .entry(player_id.to_owned())
                .or_default()
                .failed_unit_testing = true;
            self.restore_rumor(&rumor).await?;

            return Ok(Err(DebugFailure::IncorrectErrorLines));
        }

        Ok(Ok(library.zero_day()))
    }

    pub async fn debug_library_payload(
        &self,
        computer: ComputerId,
        library_path: &str,
        mem_zone: &str,
        path_file: &str,
    ) -> Result<DebugPayload> {
        let fail = |failure| Ok(DebugPayload::Failure(failure));
        let Some(mut library) = self.debug_library(computer, library_path).await? else {
            return fail(DebugFailure::MissingFile);
        };

        if path::parent(library_path) != LIBRARY_FOLDER {
            return fail(DebugFailure::OutsideLib);
        }

        if library.is_patched(self.world.time_seconds()) {
            return fail(DebugFailure::PatchPresent);
        }

        let zero_day = library.zero_day();

        if zero_day.address != mem_zone {
            return fail(DebugFailure::NoVulnerability);
        }

        let file_path = if path_file.is_empty() {
            library_path.to_owned()
        } else {
            path_file.to_owned()
        };
        let Some(file) = self.files.lookup.file(computer, &file_path, true).await? else {
            return fail(DebugFailure::UnreachableResource(file_path));
        };
        let Some(file_type) = file.data().map(|data| data.kind) else {
            return fail(DebugFailure::UnreachableResource(file_path));
        };

        let mut objects = Vec::new();

        for object in &zero_day.objects {
            let user = self.payload_user(computer, &object.access).await?;

            match object.kind {
                ZeroDayObjectKind::Computer => {
                    objects.push(PartialObject::Computer {
                        computer,
                        user,
                        methods: object.methods.clone(),
                    });
                }
                ZeroDayObjectKind::File => {
                    objects.push(PartialObject::File {
                        computer,
                        node: file.id,
                        path: file_path.clone(),
                        user: user.clone(),
                        methods: object.methods.clone(),
                    });
                    if file_type == FileKind::Library {
                        objects.push(PartialObject::Library {
                            computer,
                            path: file_path.clone(),
                            user,
                            methods: object.methods.clone(),
                        });
                    }
                }
            }
        }

        if library.num_patches > 0 {
            library.reset_num_patches();
            self.save_debug_library(&library).await?;
        }

        Ok(DebugPayload::Objects(objects))
    }
}

impl Debug {
    /// Publishes credentials through `ZeroDaySystem.AddOfferCreds`; the returned text is the game's value.
    pub async fn add_offer_creds(
        &self,
        player_id: &str,
        bank: &str,
        price: i64,
        user: &str,
        password: &str,
    ) -> Result<String> {
        if !is_rumor_time(self.world.time_seconds()) {
            return Ok(SALE_CLOSED.to_owned());
        }

        let mut rumor = self.rumor().await?;
        if rumor.offers.contains_key(player_id) {
            return Ok(ONE_OFFER_PER_PLAYER.to_owned());
        }
        let Some(credentials) = rumor.npc_passwords.get(user) else {
            return Ok(INCORRECT_OFFER_CREDENTIALS.to_owned());
        };
        if credentials.password != password {
            return Ok(INCORRECT_OFFER_CREDENTIALS.to_owned());
        }
        if credentials.player_id.is_some() {
            return Ok(USED_OFFER_CREDENTIALS.to_owned());
        }

        rumor.offers.insert(
            player_id.to_owned(),
            DebugOffer {
                seller: player_id.to_owned(),
                bank: bank.to_owned(),
                price,
                user: user.to_owned(),
                password: password.to_owned(),
                purchased: false,
            },
        );
        self.restore_rumor(&rumor).await?;

        Ok(String::new())
    }

    /// Returns the cheapest unused offer, retaining insertion order for equal prices.
    pub async fn get_best_offer_creds(&self) -> Result<Option<DebugOffer>> {
        if !is_rumor_time(self.world.time_seconds()) {
            return Ok(None);
        }

        let rumor = self.rumor().await?;
        Ok(rumor
            .offers
            .values()
            .filter(|offer| !offer.purchased)
            .filter(|offer| {
                rumor
                    .npc_passwords
                    .get(&offer.user)
                    .is_some_and(|credentials| credentials.player_id.is_none())
            })
            .min_by_key(|offer| offer.price)
            .cloned())
    }

    /// Pays the player's bank, credits the seller, then binds and marks the offer (`BuyOfferCreds`).
    pub async fn buy_offer_creds(
        &self,
        seller: &str,
        price: i64,
        buyer_id: &str,
    ) -> Result<DebugPurchase> {
        let mut rumor = self.rumor().await?;
        let Some(offer) = rumor.offers.get(seller) else {
            return Ok(DebugPurchase::Refused(OFFER_NOT_AVAILABLE.to_owned()));
        };
        if !is_rumor_time(self.world.time_seconds()) || offer.purchased || offer.price != price {
            return Ok(DebugPurchase::Refused(OFFER_NOT_AVAILABLE.to_owned()));
        }
        if rumor
            .npc_passwords
            .get(&offer.user)
            .is_none_or(|credentials| credentials.player_id.is_some())
        {
            return Ok(DebugPurchase::Refused(CREDENTIALS_IN_USE.to_owned()));
        }
        let offer = offer.clone();

        if let Err(failure) = self
            .bank
            .pay_purchase(price, "Debug Tools Credentials", false)
            .await?
        {
            return Ok(DebugPurchase::Refused(purchase_text(failure)));
        }

        if let Some(mut bank) = self.bank.bank(&offer.bank).await? {
            bank.record_transaction(BankTransaction::new(
                &offer.bank,
                offer.price,
                "Debug Tools Credentials",
                &self.world.date(),
            ));
            self.bank.save_bank(&bank).await?;
            self.bank.save_bank_transactions(&bank).await?;
        }
        rumor
            .npc_passwords
            .get_mut(&offer.user)
            .expect("validated credentials exist")
            .player_id = Some(buyer_id.to_owned());
        rumor
            .offers
            .get_mut(seller)
            .expect("validated offer exists")
            .purchased = true;
        self.restore_rumor(&rumor).await?;

        Ok(DebugPurchase::Purchased(offer))
    }

    #[must_use]
    pub fn player_id(&self) -> String {
        format!("player:{}", self.world.player().0)
    }

    pub async fn payload_user(&self, computer: ComputerId, choice: &str) -> Result<String> {
        if let Some(user) = fixed_payload_user(choice) {
            return Ok(user.to_owned());
        }

        Ok(self
            .computers
            .users
            .normal_user(computer)
            .await?
            .unwrap_or_else(|| ROOT_USER.to_owned()))
    }
}

/// A world service with only its declared dependencies.
pub struct Debug {
    pub(crate) world: greyhack_world_core::WorldRef,
    pub(crate) rows: std::rc::Rc<crate::storage::DebugRepository>,
    pub(crate) computers: greyhack_world_modules::Dep<greyhack_world_core::ComputerService>,
    pub(crate) networks: greyhack_world_modules::Dep<greyhack_world_core::NetworkService>,
    pub(crate) files: greyhack_world_modules::Dep<greyhack_world_core::FileSystemService>,
    pub(crate) npcs: greyhack_world_modules::Dep<greyhack_world_npcs::NpcService>,
    pub(crate) npcs_mail: greyhack_world_modules::Dep<greyhack_world_npcs::Mail>,
    pub(crate) bank: greyhack_world_modules::Dep<greyhack_world_economy::Bank>,
    pub(crate) generator: greyhack_world_modules::Dep<crate::Generator>,
}

impl Debug {
    #[expect(
        clippy::too_many_arguments,
        reason = "The service receives its independent repositories and domain dependencies from the module root"
    )]
    pub(crate) fn new(
        world: &greyhack_world_core::WorldRef,
        debug: std::rc::Rc<crate::storage::DebugRepository>,
        computers: greyhack_world_modules::Dep<greyhack_world_core::ComputerService>,
        networks: greyhack_world_modules::Dep<greyhack_world_core::NetworkService>,
        files: greyhack_world_modules::Dep<greyhack_world_core::FileSystemService>,
        npcs: greyhack_world_modules::Dep<greyhack_world_npcs::NpcService>,
        npcs_mail: greyhack_world_modules::Dep<greyhack_world_npcs::Mail>,
        bank: greyhack_world_modules::Dep<greyhack_world_economy::Bank>,
        generator: greyhack_world_modules::Dep<crate::Generator>,
    ) -> Self {
        Self {
            world: world.clone(),
            rows: debug,
            computers,
            networks,
            files,
            npcs,
            npcs_mail,
            bank,
            generator,
        }
    }
}

fn game_text(key: &str) -> String {
    GameTextTable::english().get(key).unwrap_or(key).to_owned()
}

fn purchase_text(failure: PurchaseFailure) -> String {
    match failure {
        PurchaseFailure::NoBankAccount => {
            "You must register a bank account in order to buy".to_owned()
        }
        PurchaseFailure::UnknownBankAccount(user) => format!("{user} user not found"),
        PurchaseFailure::InsufficientFunds => {
            "The purchase could not be completed. Insufficient funds".to_owned()
        }
    }
}
