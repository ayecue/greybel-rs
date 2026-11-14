//! The hack shop's catalog: the exploits of a library version (`VersionsControl.GeneraExploit`,
//! `NetworkHelperServer.GetExploitListServerRpc`).

use std::cmp::Ordering;

use greyhack_behaviour::{
    ExploitPart, LibraryKind, compare_versions, exploit_part_choices, is_valid_version,
    unique_exploit_name,
};

use super::failure::ExploitShopFailure;
use crate::{Exploit, ExploitOffer, Library, LibraryId, Result, Rng, Shop, Word};

/// Methods of [`World`] for exploits shop catalog.
impl Shop {
    pub async fn exploit_offers(
        &self,
        library: LibraryKind,
        version: &str,
    ) -> Result<std::result::Result<Vec<ExploitOffer>, ExploitShopFailure>> {
        if !is_valid_version(version) {
            return Ok(Err(ExploitShopFailure::InvalidVersion));
        }

        let Some((id, library)) = self.shop_library(library, version).await? else {
            return Ok(Err(ExploitShopFailure::NoResults));
        };
        let exploits = self.library_exploits(id, &library).await?;
        let offers: Vec<ExploitOffer> = exploits
            .iter()
            .filter_map(|exploit| {
                let description = exploit.description(&library)?;
                let price = exploit.price(&library)?;

                Some(ExploitOffer {
                    name: exploit.name.clone(),
                    description,
                    price,
                })
            })
            .collect();

        if offers.is_empty() {
            return Ok(Err(ExploitShopFailure::NoResults));
        }

        Ok(Ok(offers))
    }

    pub async fn set_library_exploits(
        &self,
        library: LibraryKind,
        version: &str,
        exploits: &[Exploit],
    ) -> Result<bool> {
        let Some(id) = self.files.library_of(library, version).await? else {
            return Ok(false);
        };

        self.exploits.replace_library_exploits(id, exploits).await?;

        Ok(true)
    }

    pub async fn patch_exploit_description(
        &self,
        library: LibraryKind,
        version: &str,
        zone_address: &str,
        unsec_value: &str,
    ) -> Result<bool> {
        let Some((id, library)) = self.shop_library(library, version).await? else {
            return Ok(false);
        };
        let exploits = self.library_exploits(id, &library).await?;
        // The game looks for the overflow call in the script, which a password exploit's call (with the new password as
        // a third argument) never contains, so the description of that exploit is never patched.
        let call = format!(".overflow(\"{zone_address}\", \"{unsec_value}\")");
        let Some(exploit) = exploits.iter().find(|exploit| {
            exploit
                .source(&library)
                .is_some_and(|source| source.contains(&call))
        }) else {
            return Ok(false);
        };

        self.exploits
            .patch_library_exploit(id, &exploit.name)
            .await?;

        Ok(true)
    }

    pub async fn shop_library(
        &self,
        kind: LibraryKind,
        version: &str,
    ) -> Result<Option<(LibraryId, Library)>> {
        let Some(id) = self.files.library_of(kind, version).await? else {
            return Ok(None);
        };

        Ok(self
            .libraries
            .library(id)
            .await?
            .map(|library| (id, library)))
    }

    pub async fn library_exploits(&self, id: LibraryId, library: &Library) -> Result<Vec<Exploit>> {
        for hide in [false, true] {
            if self.exploits.has_exploit_batch(id, hide).await? {
                continue;
            }

            if hide && !self.is_replaced(library).await? {
                continue;
            }

            self.make_exploits(id, library, hide).await?;
        }

        self.exploits.library_exploits(id).await
    }
}

impl Shop {
    pub async fn is_replaced(&self, library: &Library) -> Result<bool> {
        if library.kind == LibraryKind::KernelRouter {
            return Ok(false);
        }

        Ok(
            match self.generator.latest_full_library(library.kind).await? {
                Some(latest) => {
                    compare_versions(&library.version, &latest.version) == Ordering::Less
                }
                None => false,
            },
        )
    }

    pub async fn make_exploits(&self, id: LibraryId, library: &Library, hide: bool) -> Result<()> {
        let mut siblings = self.exploits.library_exploits(id).await?;
        let mut rng = Rng::new(self.world.world_seed().derive_u64(
            "exploit-shop-v1",
            &[
                &library.kind.code().to_string(),
                &library.version,
                if hide { "hidden" } else { "visible" },
            ],
        ));
        let mut batch = Vec::new();

        for zone in library.zones.iter().filter(|zone| zone.hide == hide) {
            for weakness in &zone.weaknesses {
                let base = Word::Exploit.generate(&mut rng).to_lowercase();
                let name = unique_exploit_name(&base, |name| {
                    siblings.iter().any(|exploit| exploit.name == name)
                });
                let choices = exploit_part_choices(weakness.reward);
                let part = if choices.is_empty() {
                    ExploitPart::BaseScript
                } else {
                    *rng.pick(choices)
                };
                let exploit = Exploit {
                    name,
                    hidden: hide,
                    patched: false,
                    zone_address: zone.address.clone(),
                    unsec_value: weakness.unsec_value.clone(),
                    part,
                };
                let source = exploit.source(library);

                if siblings.iter().any(|other| other.source(library) == source) {
                    continue;
                }

                siblings.push(exploit.clone());
                batch.push(exploit);
            }
        }

        self.exploits.add_library_exploits(id, hide, &batch).await
    }
}
