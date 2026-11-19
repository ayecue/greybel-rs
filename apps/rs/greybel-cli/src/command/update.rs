use clap::Args;

use crate::report::Failure;
use crate::update::{SelfUpdate, UpdateCheck};

#[derive(Debug, Args)]
pub struct UpdateCommand {
    /// Only report whether a newer version is available.
    #[arg(long)]
    check: bool,
}

impl UpdateCommand {
    pub fn run(self) -> Result<(), Failure> {
        let latest = UpdateCheck::latest()?;

        if !latest.is_newer_than(UpdateCheck::VERSION) {
            println!("greybel {} is the newest version.", UpdateCheck::VERSION);

            return Ok(());
        }

        if self.check {
            println!(
                "greybel {} is available (this is {}): run `greybel update`.",
                latest.version,
                UpdateCheck::VERSION
            );

            return Ok(());
        }

        let asset = SelfUpdate::asset(&latest)?;

        println!("Downloading greybel {} from {}", latest.version, asset.url);
        SelfUpdate::new().install(asset, UpdateCheck::USER_AGENT)?;
        println!(
            "Updated greybel from {} to {}.",
            UpdateCheck::VERSION,
            latest.version
        );

        Ok(())
    }
}
