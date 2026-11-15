//! Debug authentication, library inspection, patching, tests, and zero-day payloads.

use greyhack_behaviour::{
    FileKind, LIBRARY_FOLDER, MAX_DEBUG_LOGIN_ATTEMPTS, ROOT_USER, error_lines_match,
    fixed_payload_user, format_auto_patch_date, path,
};
use greyhack_world_core::{ComputerId, NodeId, Result};

use crate::generation::challenge::normalized_code;
use crate::{DebugAccount, DebugChallenge, DebugLibraryState, ZeroDay, ZeroDayObjectKind};

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
    /// The password is invalid or the debug account does not exist.
    IncorrectPassword,
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
    pub async fn debug_account(&self, user: &str) -> Result<Option<DebugAccount>> {
        self.rows.debug_account(user).await
    }

    pub async fn save_debug_account(&self, account: &DebugAccount) -> Result<()> {
        self.rows.set_debug_account(account).await
    }

    pub async fn login_debug(
        &self,
        user: &str,
        password: &str,
    ) -> Result<Result<(), DebugFailure>> {
        let Some(mut account) = self.debug_account(user).await? else {
            return Ok(Err(DebugFailure::IncorrectPassword));
        };

        if account.failed_logins >= MAX_DEBUG_LOGIN_ATTEMPTS {
            return Ok(Err(DebugFailure::LoginLockout));
        }

        let outcome = if account.check_password(password) {
            account.failed_logins = 0;

            Ok(())
        } else {
            account.failed_logins += 1;

            Err(DebugFailure::IncorrectPassword)
        };

        self.save_debug_account(&account).await?;

        Ok(outcome)
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
        user: &str,
        error_lines: &[i64],
    ) -> Result<Result<ZeroDay, DebugFailure>> {
        if self
            .debug_account(user)
            .await?
            .is_some_and(|account| account.unit_test_failed)
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
            if let Some(mut account) = self.debug_account(user).await? {
                account.unit_test_failed = true;
                self.save_debug_account(&account).await?;
            }

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
    pub(crate) files: greyhack_world_modules::Dep<greyhack_world_core::FileSystemService>,
}

impl Debug {
    pub(crate) fn new(
        world: &greyhack_world_core::WorldRef,
        debug: std::rc::Rc<crate::storage::DebugRepository>,
        computers: greyhack_world_modules::Dep<greyhack_world_core::ComputerService>,
        files: greyhack_world_modules::Dep<greyhack_world_core::FileSystemService>,
    ) -> Self {
        Self {
            world: world.clone(),
            rows: debug,
            computers,
            files,
        }
    }
}
