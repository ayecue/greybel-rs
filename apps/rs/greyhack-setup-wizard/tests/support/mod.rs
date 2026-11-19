//! A fake game for the window.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use egui_kittest::Harness;
use greyhack_setup::Platform;
use greyhack_setup::game::GameLayout;
use greyhack_setup_wizard::window::SetupWindow;

/// A fake macOS game in a temporary folder, removed when dropped.
pub struct FakeGame {
    folder: PathBuf,
}

impl FakeGame {
    pub fn new() -> Self {
        static COUNT: AtomicUsize = AtomicUsize::new(0);

        let folder = std::env::temp_dir().join(format!(
            "greyhack-setup-window-{}-{}",
            std::process::id(),
            COUNT.fetch_add(1, Ordering::Relaxed)
        ));

        std::fs::create_dir_all(folder.join("Grey Hack.app/Contents/Resources/Data/Managed"))
            .expect("the game");

        Self { folder }
    }

    pub fn write(&self, path: &str) -> &Self {
        let full = self.folder.join(path);

        std::fs::create_dir_all(full.parent().expect("a folder")).expect("the folder");
        std::fs::write(full, "").expect("the file");

        self
    }

    pub fn path(&self) -> &Path {
        &self.folder
    }

    pub fn window(&self) -> Harness<'static, SetupWindow> {
        let game = GameLayout::from_folder(&self.folder).expect("the game");

        Harness::builder()
            .with_size([640.0, 900.0])
            .build_eframe(move |context| {
                SetupWindow::apply_style(&context.egui_ctx);

                SetupWindow::with_game(Platform::MacOS, game)
            })
    }
}

impl Drop for FakeGame {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.folder);
    }
}
