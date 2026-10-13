//! Grey Hack-compatible domain resolution (`IpGenerator.UniqueNameToIP`).

use greyhack_behaviour::{WEB_PREFIX, unique_name_to_ip};

use crate::{Contact, Result, World};

impl World {
    /// Resolves web `domain`, prepending `www.` if absent (`IpGenerator.UniqueNameToIP`). Uses a
    /// registered network address, then an address encoded by the domain, or `None`.
    ///
    /// # Errors
    ///
    /// Returns any database lookup error.
    pub async fn resolve_domain(&self, domain: &str) -> Result<Option<String>> {
        let address = if domain.starts_with(WEB_PREFIX) {
            domain.to_owned()
        } else {
            format!("{WEB_PREFIX}{domain}")
        };

        // Network web addresses are `www.` plus the domain (`ServerMap.RouterNode.GetDomain`).
        Ok(self
            .lookup(&address[WEB_PREFIX.len()..])
            .await?
            .or_else(|| unique_name_to_ip(&self.world_seed, &address)))
    }

    /// Returns `whois` domain and administrative contact for `public_ip`, or `None` without a
    /// network.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn whois(&mut self, public_ip: &str) -> Result<Option<(String, Option<Contact>)>> {
        let router = match self.router_or_generate(public_ip).await? {
            Some(router) => self.db.router(router).await?,
            None => None,
        };

        // `whois` displays `www.` plus the domain (`ServerMap.RouterNode.GetWebAddress`).
        Ok(router.map(|router| {
            let address = if router.domain.is_empty() {
                router.domain
            } else {
                format!("{WEB_PREFIX}{}", router.domain)
            };

            (address, router.contact)
        }))
    }
}
