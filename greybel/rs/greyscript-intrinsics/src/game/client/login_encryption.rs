use greybel_interpreter::{FromVm, Vm};

use super::game::Game;
use crate::error::IntrinsicError;

/// Async-native host parameter indicating a server-side SSH encode/decode program ([`Game::encrypts_login`]).
#[derive(Debug, Clone, Copy)]
pub(crate) struct LoginEncryption(bool);

impl LoginEncryption {
    /// Whether this program encrypts a login.
    pub(crate) fn is_active(self) -> bool {
        self.0
    }

    /// Rejects `intrinsic` during login encryption.
    /// Matches Grey Hack's restrictions on intrinsics that access state beyond the script (`HelperScript.IsPendingSshSec`).
    pub(crate) fn deny(self, intrinsic: &'static str) -> Result<(), IntrinsicError> {
        if self.0 {
            Err(IntrinsicError::LoginEncryption(intrinsic))
        } else {
            Ok(())
        }
    }

    /// Rejects `intrinsic` when the VM's current program encrypts a login ([`Self::deny`]).
    pub(crate) fn deny_in(vm: &Vm, intrinsic: &'static str) -> Result<(), IntrinsicError> {
        Self::from_vm(vm).deny(intrinsic)
    }
}

impl FromVm for LoginEncryption {
    fn from_vm(vm: &Vm) -> Self {
        Self(
            vm.host_data::<Game>()
                .is_some_and(|game| game.encrypts_login),
        )
    }
}
