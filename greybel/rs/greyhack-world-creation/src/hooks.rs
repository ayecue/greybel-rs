//! Creation's implementations of core and NPC upward calls.

use greyhack_world_core::{
    ComputerId, HookFuture, NetworkGeneration, PersonId, RouterId, WorldRef,
};

use crate::{ComputerOptions, NetworkGeneration as NetworkCreator, NetworkOptions, Restores};

impl NetworkGeneration for NetworkCreator {
    fn router_or_generate<'a>(
        &'a self,
        _: &'a WorldRef,
        public_ip: &'a str,
    ) -> HookFuture<'a, Option<RouterId>> {
        Box::pin(async move { self.router_or_generate(public_ip).await })
    }
}

impl greyhack_world_npcs::NetworkCreation for NetworkCreator {
    fn add_network<'a>(&'a self, public_ip: &'a str) -> HookFuture<'a, RouterId> {
        Box::pin(async move {
            NetworkCreator::add_network(
                self,
                &NetworkOptions {
                    public_ip: Some(public_ip.to_owned()),
                    ..NetworkOptions::default()
                },
            )
            .await
        })
    }

    fn add_phone<'a>(
        &'a self,
        owner: ComputerId,
        npc: PersonId,
        user: &'a str,
        password: &'a str,
    ) -> HookFuture<'a, Option<ComputerId>> {
        Box::pin(async move {
            NetworkCreator::add_phone(
                self,
                owner,
                npc,
                &ComputerOptions {
                    users: Some(vec![(user.to_owned(), password.to_owned())]),
                    ..ComputerOptions::default()
                },
            )
            .await
        })
    }
}

impl greyhack_world_npcs::NetworkRestore for Restores {
    fn restore_admin_network(&self, admin_computer: ComputerId) -> HookFuture<'_, ()> {
        Box::pin(async move { Restores::restore_admin_network(self, admin_computer).await })
    }
}
