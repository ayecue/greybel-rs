//! The window: the game's status, a choice for BepInEx, the hook and the port, and a review of the changes before
//! anything is downloaded or changed.

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::mpsc::{Receiver, Sender, channel};

use eframe::egui::{self, Color32, RichText};
use greyhack_setup::bepinex::{BepInExMajor, BepInExRelease};
use greyhack_setup::game::{GameLayout, GameLocator};
use greyhack_setup::hook::{HookRelease, MessageHook};
use greyhack_setup::{
    BepInExSource, Downloader, GameStatus, HookSource, Platform, Readiness, Setup, SetupResult,
};

pub struct SetupWindow {
    platform: Platform,
    game: Option<GameLayout>,
    status: Option<GameStatus>,
    locate_failure: Option<String>,
    bepinex: BepInExChoice,
    hook: HookChoice,
    port: String,
    remove_steam_api: bool,
    is_review_open: bool,
    setup_events: Option<Receiver<SetupEvent>>,
    log: Vec<LogLine>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum BepInExChoice {
    Retain,
    Install(BepInExMajor),
    Zip(Option<PathBuf>),
    Remove,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum HookChoice {
    Retain,
    Release,
    File(Option<PathBuf>),
    Remove,
}

/// One step of the review, run in this order.
#[derive(Debug, Clone, PartialEq, Eq)]
enum SetupChange {
    InstallBepInEx(BepInExMajor),
    InstallBepInExZip(PathBuf),
    RemoveBepInEx,
    InstallHookRelease(Option<BepInExMajor>),
    InstallHookFile(Option<BepInExMajor>, PathBuf),
    RemoveHook,
    SetPort(u16),
    RemoveSteamApi,
}

enum SetupEvent {
    Progress(String),
    Failure(String),
    Completion,
}

struct LogLine {
    text: String,
    is_failure: bool,
}

impl SetupWindow {
    const FAILURE: Color32 = Color32::from_rgb(230, 120, 80);
    const LABEL_WIDTH: f32 = 96.0;
    const OPTION_WIDTH: f32 = 300.0;
    const SUCCESS: Color32 = Color32::from_rgb(90, 190, 110);
    const WARNING: Color32 = Color32::from_rgb(230, 170, 60);

    pub fn open() -> ExitCode {
        let options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_title("Grey Hack Setup")
                .with_inner_size([640.0, 720.0])
                .with_min_inner_size([560.0, 480.0]),
            ..eframe::NativeOptions::default()
        };
        let result = eframe::run_native(
            "Grey Hack Setup",
            options,
            Box::new(|context| {
                Self::apply_style(&context.egui_ctx);

                Ok(Box::new(Self::new()))
            }),
        );

        if result.is_ok() {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        }
    }

    /// The window for the game found in `GREY_HACK_PATH` or the Steam libraries.
    fn new() -> Self {
        let mut window = Self::with_platform(Platform::current().unwrap_or(Platform::Linux));

        window.locate();

        window
    }

    #[must_use]
    pub fn with_game(platform: Platform, game: GameLayout) -> Self {
        let mut window = Self::with_platform(platform);

        window.select_game(game);

        window
    }

    fn with_platform(platform: Platform) -> Self {
        Self {
            platform,
            game: None,
            status: None,
            locate_failure: None,
            bepinex: BepInExChoice::Retain,
            hook: HookChoice::Retain,
            port: MessageHook::DEFAULT_PORT.to_string(),
            remove_steam_api: false,
            is_review_open: false,
            setup_events: None,
            log: Vec::new(),
        }
    }

    /// Finds the game in `GREY_HACK_PATH` or the Steam libraries.
    fn locate(&mut self) {
        match GameLocator::new(self.platform).locate() {
            Ok(Some(game)) => self.select_game(game),
            Ok(None) => {
                self.locate_failure = Some(
                    "Grey Hack wasn't found in your Steam libraries. Choose its folder.".to_owned(),
                );
            }
            Err(error) => self.locate_failure = Some(error.to_string()),
        }
    }

    fn choose_folder(&mut self) {
        let Some(folder) = rfd::FileDialog::new()
            .set_title("Choose the Grey Hack folder")
            .pick_folder()
        else {
            return;
        };

        match GameLayout::from_folder(folder) {
            Ok(game) => self.select_game(game),
            Err(error) => self.locate_failure = Some(error.to_string()),
        }
    }

    /// Shows `game`'s status, with the choices that complete its setup selected.
    fn select_game(&mut self, game: GameLayout) {
        let status = GameStatus::read(&game, self.platform);

        self.bepinex = if status.bepinex.is_some() {
            BepInExChoice::Retain
        } else {
            BepInExChoice::Install(BepInExMajor::Stable)
        };
        self.hook = if matches!(status.readiness, Readiness::Ready { .. }) {
            HookChoice::Retain
        } else {
            HookChoice::Release
        };
        self.port = status.port.unwrap_or(MessageHook::DEFAULT_PORT).to_string();
        self.remove_steam_api = status.duplicate_steam_api.is_some();
        self.locate_failure = None;
        self.status = Some(status);
        self.game = Some(game);
    }

    fn refresh(&mut self) {
        if let Some(game) = &self.game {
            self.status = Some(GameStatus::read(game, self.platform));
        }
    }

    /// The BepInEx major the hook is for: the one being installed, else the installed one.
    fn hook_major(&self) -> Option<BepInExMajor> {
        match &self.bepinex {
            BepInExChoice::Install(major) => Some(*major),
            BepInExChoice::Zip(_) | BepInExChoice::Remove => None,
            BepInExChoice::Retain => self
                .status
                .as_ref()
                .and_then(|status| status.bepinex.as_ref())
                .map(|installation| installation.major),
        }
    }

    fn will_have_bepinex(&self) -> bool {
        match &self.bepinex {
            BepInExChoice::Install(_) | BepInExChoice::Zip(_) => true,
            BepInExChoice::Remove => false,
            BepInExChoice::Retain => self
                .status
                .as_ref()
                .is_some_and(|status| status.bepinex.is_some()),
        }
    }

    /// The changes the choices make, or why they can't be applied.
    fn changes(&self) -> Result<Vec<SetupChange>, String> {
        let Some(status) = &self.status else {
            return Err("Choose the game's folder first.".to_owned());
        };
        let mut changes = Vec::new();

        match &self.bepinex {
            BepInExChoice::Retain => {}
            BepInExChoice::Install(major) => changes.push(SetupChange::InstallBepInEx(*major)),
            BepInExChoice::Zip(Some(zip)) => {
                changes.push(SetupChange::InstallBepInExZip(zip.clone()))
            }
            BepInExChoice::Zip(None) => return Err("Choose a BepInEx zip file.".to_owned()),
            BepInExChoice::Remove => changes.push(SetupChange::RemoveBepInEx),
        }

        match &self.hook {
            HookChoice::Retain => {}
            HookChoice::Remove => changes.push(SetupChange::RemoveHook),
            _ if !self.will_have_bepinex() => {
                return Err("The message hook needs BepInEx, which won't be installed.".to_owned());
            }
            HookChoice::Release => changes.push(SetupChange::InstallHookRelease(self.hook_major())),
            HookChoice::File(None) => return Err("Choose the message hook's file.".to_owned()),
            HookChoice::File(Some(file)) => {
                let major = self.hook_major();

                if let Some(major) = major
                    && file
                        .file_stem()
                        .is_none_or(|stem| stem != MessageHook::assembly_name(major))
                {
                    return Err(format!(
                        "{} is the wrong file. BepInEx {} needs {}.dll.",
                        file.display(),
                        major.number(),
                        MessageHook::assembly_name(major)
                    ));
                }

                changes.push(SetupChange::InstallHookFile(major, file.clone()));
            }
        }

        let port = self
            .port
            .trim()
            .parse::<u16>()
            .ok()
            .filter(|&port| port != 0)
            .ok_or_else(|| "Enter a port: a whole number from 1 to 65535.".to_owned())?;

        if status.port.unwrap_or(MessageHook::DEFAULT_PORT) != port {
            changes.push(SetupChange::SetPort(port));
        }

        if self.remove_steam_api && status.duplicate_steam_api.is_some() {
            changes.push(SetupChange::RemoveSteamApi);
        }

        Ok(changes)
    }

    /// Runs `changes` on a background thread and reports progress to the log.
    fn apply(&mut self, changes: Vec<SetupChange>) {
        let Some(game) = self.game.clone() else {
            return;
        };
        let (sender, receiver) = channel();
        let platform = self.platform;

        self.log.clear();
        self.setup_events = Some(receiver);
        std::thread::spawn(move || {
            let downloader = Downloader::new(Downloader::default_cache_folder(platform));
            let setup = Setup::new(game, platform, &downloader);
            let result = changes
                .iter()
                .try_for_each(|change| change.run(&setup, &downloader, &sender));
            let event = match result {
                Ok(()) => SetupEvent::Completion,
                Err(error) => SetupEvent::Failure(error.to_string()),
            };
            let _ = sender.send(event);
        });
    }

    fn receive(&mut self) {
        let Some(setup_events) = &self.setup_events else {
            return;
        };

        while let Ok(event) = setup_events.try_recv() {
            match event {
                SetupEvent::Progress(text) => self.log.push(LogLine {
                    text,
                    is_failure: false,
                }),
                SetupEvent::Failure(text) => {
                    self.log.push(LogLine {
                        text: format!("Failure: {text}"),
                        is_failure: true,
                    });
                    self.setup_events = None;
                    self.refresh();

                    return;
                }
                SetupEvent::Completion => {
                    self.log.push(LogLine {
                        text: "Setup complete.".to_owned(),
                        is_failure: false,
                    });
                    self.setup_events = None;

                    if let Some(game) = self.game.clone() {
                        self.select_game(game);
                    }

                    return;
                }
            }
        }
    }

    /// The window's spacing: more room around buttons and between rows than egui's defaults.
    pub fn apply_style(ctx: &egui::Context) {
        ctx.all_styles_mut(|style| {
            style.spacing.item_spacing = egui::vec2(8.0, 6.0);
            style.spacing.button_padding = egui::vec2(10.0, 4.0);
            style.spacing.interact_size.y = 22.0;
        });
    }

    /// A titled section with a frame; `add_controls` adds controls right-aligned beside the title.
    fn card(
        ui: &mut egui::Ui,
        title: &str,
        add_controls: impl FnOnce(&mut egui::Ui),
        add_contents: impl FnOnce(&mut egui::Ui),
    ) {
        egui::Frame::group(ui.style())
            .fill(ui.visuals().faint_bg_color)
            .corner_radius(6.0)
            .inner_margin(egui::Margin::same(12))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.set_min_height(ui.spacing().interact_size.y + 4.0);
                    ui.label(RichText::new(title).strong().size(15.0));
                    ui.with_layout(
                        egui::Layout::right_to_left(egui::Align::Center),
                        add_controls,
                    );
                });
                ui.add_space(4.0);
                add_contents(ui);
            });
        ui.add_space(8.0);
    }

    /// A radio button in a column of [`Self::OPTION_WIDTH`], so the controls `add_controls` adds line up.
    fn option(
        ui: &mut egui::Ui,
        is_selected: bool,
        text: impl Into<egui::WidgetText>,
        add_controls: impl FnOnce(&mut egui::Ui),
    ) -> egui::Response {
        ui.horizontal(|ui| {
            let height = ui.spacing().interact_size.y;
            let response = ui
                .allocate_ui_with_layout(
                    egui::vec2(Self::OPTION_WIDTH, height),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| ui.radio(is_selected, text),
                )
                .inner;

            add_controls(ui);

            response
        })
        .inner
    }

    /// A note under an option, indented to its text.
    fn note(ui: &mut egui::Ui, text: impl Into<String>) {
        ui.horizontal(|ui| {
            ui.add_space(ui.spacing().icon_width + ui.spacing().icon_spacing);
            ui.add(egui::Label::new(RichText::new(text).size(12.5).weak()).wrap());
        });
    }

    /// A path in one line, shortened with `…` where it doesn't fit; the complete path shows on hover.
    fn path_label(ui: &mut egui::Ui, path: &std::path::Path) {
        let text = path.display().to_string();

        ui.add(egui::Label::new(RichText::new(&text).monospace().size(12.0)).truncate())
            .on_hover_text(text);
    }

    fn show_header(ui: &mut egui::Ui) {
        ui.add_space(4.0);
        ui.label(RichText::new("Grey Hack Setup").heading().strong());
        ui.label(
            RichText::new(
                "Installs the message hook, the BepInEx plugin that connects Greybel to Grey Hack.",
            )
            .weak(),
        );
        ui.add_space(4.0);
    }

    fn show_status(&mut self, ui: &mut egui::Ui) {
        let mut action = None;

        Self::card(
            ui,
            "Game",
            |ui| {
                if ui.button("Search again").clicked() {
                    action = Some(false);
                }

                if ui.button("Choose folder…").clicked() {
                    action = Some(true);
                }
            },
            |ui| {
                let absent = || "Not installed".to_owned();

                egui::Grid::new("status")
                    .num_columns(2)
                    .spacing([16.0, 6.0])
                    .min_col_width(Self::LABEL_WIDTH)
                    .show(ui, |ui| {
                        ui.label(RichText::new("Folder").weak());

                        match &self.game {
                            Some(game) => Self::path_label(ui, game.folder()),
                            None => {
                                ui.label("Not found");
                            }
                        }

                        ui.end_row();

                        let Some(status) = &self.status else {
                            return;
                        };

                        ui.label(RichText::new("BepInEx").weak());
                        ui.label(
                            status
                                .bepinex
                                .as_ref()
                                .map_or_else(absent, |installation| installation.description()),
                        );
                        ui.end_row();
                        ui.label(RichText::new("Message hook").weak());
                        ui.label(status.hook.clone().unwrap_or_else(absent));
                        ui.end_row();
                        ui.label(RichText::new("Port").weak());
                        ui.label(status.port.map_or_else(
                            || format!("{} (default)", MessageHook::DEFAULT_PORT),
                            |port| port.to_string(),
                        ));
                        ui.end_row();
                    });

                if let Some(failure) = &self.locate_failure {
                    ui.add_space(4.0);
                    ui.colored_label(Self::FAILURE, failure);
                }

                let Some(status) = &self.status else {
                    return;
                };
                let color = if matches!(status.readiness, Readiness::Ready { .. }) {
                    Self::SUCCESS
                } else {
                    Self::WARNING
                };

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    let (dot, _) =
                        ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());

                    ui.painter().circle_filled(dot.center(), 4.0, color);
                    ui.colored_label(color, status.readiness_text());
                });

                if let Some(option) = &status.launch_option {
                    ui.add_space(4.0);
                    ui.label("Start the game from Steam with this launch option (Properties, Launch Options):");
                    egui::Frame::new()
                        .fill(ui.visuals().extreme_bg_color)
                        .corner_radius(4.0)
                        .inner_margin(egui::Margin::symmetric(8, 6))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.horizontal(|ui| {
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if ui.small_button("Copy").clicked() {
                                            ui.ctx().copy_text(option.clone());
                                        }

                                        ui.with_layout(
                                            egui::Layout::left_to_right(egui::Align::Center),
                                            |ui| {
                                                ui.add(
                                                    egui::Label::new(
                                                        RichText::new(option)
                                                            .monospace()
                                                            .size(12.0),
                                                    )
                                                    .wrap(),
                                                );
                                            },
                                        );
                                    },
                                );
                            });
                        });
                }
            },
        );

        match action {
            Some(true) => self.choose_folder(),
            Some(false) => self.locate(),
            None => {}
        }
    }

    fn show_choices(&mut self, ui: &mut egui::Ui) {
        let Some(status) = self.status.clone() else {
            return;
        };
        let stable = BepInExRelease::of(BepInExMajor::Stable, self.platform);
        let beta = BepInExRelease::of(BepInExMajor::Beta, self.platform);

        Self::card(
            ui,
            "BepInEx",
            |_| {},
            |ui| {
                if status.bepinex.is_some()
                    && Self::option(
                        ui,
                        self.bepinex == BepInExChoice::Retain,
                        "Keep the installed BepInEx",
                        |_| {},
                    )
                    .clicked()
                {
                    self.bepinex = BepInExChoice::Retain;
                }

                for (release, text) in [
                    (
                        &stable,
                        format!("Install BepInEx 5 ({}, recommended)", stable.version),
                    ),
                    (&beta, format!("Install BepInEx 6 ({}, beta)", beta.version)),
                ] {
                    let choice = BepInExChoice::Install(release.major);

                    if Self::option(ui, self.bepinex == choice, text, |_| {}).clicked() {
                        self.bepinex = choice;
                    }
                }

                if self.bepinex == BepInExChoice::Install(BepInExMajor::Beta)
                    && self.platform == Platform::MacOS
                {
                    Self::note(
                        ui,
                        "On Apple Silicon Macs, Grey Hack then runs under Rosetta: BepInEx 6 for macOS supports Intel only.",
                    );
                }

                let zip = match &self.bepinex {
                    BepInExChoice::Zip(zip) => zip.clone(),
                    _ => None,
                };
                let mut file_choice = None;
                let response = Self::option(
                    ui,
                    matches!(self.bepinex, BepInExChoice::Zip(_)),
                    "Install from a zip file",
                    |ui| {
                        if ui.button("Choose file…").clicked() {
                            file_choice = rfd::FileDialog::new()
                                .set_title("Choose a BepInEx zip file")
                                .add_filter("Zip files", &["zip"])
                                .pick_file();
                        }

                        if let Some(zip) = &zip {
                            Self::path_label(ui, zip);
                        }
                    },
                );

                if let Some(file) = file_choice {
                    self.bepinex = BepInExChoice::Zip(Some(file));
                } else if response.clicked() {
                    self.bepinex = BepInExChoice::Zip(zip);
                }

                if status.bepinex.is_some()
                    && Self::option(
                        ui,
                        self.bepinex == BepInExChoice::Remove,
                        "Remove BepInEx",
                        |_| {},
                    )
                    .clicked()
                {
                    self.bepinex = BepInExChoice::Remove;
                }

                if self.bepinex == BepInExChoice::Remove || status.bepinex.is_some() {
                    Self::note(
                        ui,
                        "Installing or removing BepInEx doesn't change its plugins and settings.",
                    );
                }
            },
        );

        Self::card(
            ui,
            "Message hook",
            |_| {},
            |ui| {
                if status.hook.is_some()
                    && Self::option(
                        ui,
                        self.hook == HookChoice::Retain,
                        "Keep the installed message hook",
                        |_| {},
                    )
                    .clicked()
                {
                    self.hook = HookChoice::Retain;
                }

                if Self::option(
                    ui,
                    self.hook == HookChoice::Release,
                    format!("Download {}", MessageHook::release_tag()),
                    |_| {},
                )
                .clicked()
                {
                    self.hook = HookChoice::Release;
                }

                let file = match &self.hook {
                    HookChoice::File(file) => file.clone(),
                    _ => None,
                };
                let mut file_choice = None;
                let response = Self::option(
                    ui,
                    matches!(self.hook, HookChoice::File(_)),
                    "Install from a file",
                    |ui| {
                        if ui.button("Choose file…").clicked() {
                            file_choice = rfd::FileDialog::new()
                                .set_title("Choose the message hook's file")
                                .add_filter("Plugin files", &["dll"])
                                .pick_file();
                        }

                        if let Some(file) = &file {
                            Self::path_label(ui, file);
                        }
                    },
                );

                if let Some(file) = file_choice {
                    self.hook = HookChoice::File(Some(file));
                } else if response.clicked() {
                    self.hook = HookChoice::File(file);
                }

                if status.hook.is_some()
                    && Self::option(
                        ui,
                        self.hook == HookChoice::Remove,
                        "Remove the message hook",
                        |_| {},
                    )
                    .clicked()
                {
                    self.hook = HookChoice::Remove;
                }

                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    let label = ui.label("Port");

                    ui.add(egui::TextEdit::singleline(&mut self.port).desired_width(70.0))
                        .labelled_by(label.id);
                    ui.label(RichText::new("the message hook listens on, on this computer").weak());
                });

                if let Ok(port) = self.port.trim().parse::<u16>()
                    && status.port.unwrap_or(MessageHook::DEFAULT_PORT) != port
                    && MessageHook::is_port_in_use(port)
                {
                    ui.colored_label(
                        Self::WARNING,
                        format!("Port {port} is already used on this computer."),
                    );
                }
            },
        );

        if let Some(path) = &status.duplicate_steam_api {
            Self::card(
                ui,
                "Steam library",
                |_| {},
                |ui| {
                    ui.checkbox(
                        &mut self.remove_steam_api,
                        "Remove the duplicate libsteam_api.dylib",
                    );
                    Self::note(
                        ui,
                        format!(
                            "{} crashes Grey Hack under BepInEx, because Steam's library is then found twice.",
                            path.display()
                        ),
                    );
                },
            );
        }
    }

    /// The bar at the bottom: the progress of the changes, what stops them, and **Apply…**.
    fn show_actions(&mut self, ui: &mut egui::Ui) {
        let changes = self.changes();

        if !self.log.is_empty() {
            egui::ScrollArea::vertical()
                .max_height(96.0)
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());

                    for line in &self.log {
                        let text = RichText::new(&line.text).monospace().small();

                        ui.add(
                            egui::Label::new(if line.is_failure {
                                text.color(Self::FAILURE)
                            } else {
                                text
                            })
                            .wrap(),
                        );
                    }
                });
            ui.separator();
        }

        let can_apply = self.setup_events.is_none()
            && changes.as_ref().is_ok_and(|changes| !changes.is_empty());

        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let apply = egui::Button::new(RichText::new("Apply…").strong())
                    .min_size(egui::vec2(96.0, 28.0));

                if ui.add_enabled(can_apply, apply).clicked() {
                    self.is_review_open = true;
                }

                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    if self.setup_events.is_some() {
                        ui.spinner();
                        ui.label("Applying the changes…");
                    } else {
                        match &changes {
                            Err(problem) => {
                                ui.add(
                                    egui::Label::new(RichText::new(problem).color(Self::FAILURE))
                                        .wrap(),
                                );
                            }
                            Ok(changes) if changes.is_empty() => {
                                ui.label(RichText::new("Nothing to change.").weak());
                            }
                            Ok(changes) => {
                                ui.label(
                                    RichText::new(match changes.len() {
                                        1 => "1 change to apply.".to_owned(),
                                        count => format!("{count} changes to apply."),
                                    })
                                    .weak(),
                                );
                            }
                        }
                    }
                });
            });
        });
    }

    fn show_review(&mut self, ctx: &egui::Context) {
        let Ok(changes) = self.changes() else {
            self.is_review_open = false;

            return;
        };
        let mut decision = None;

        egui::Modal::new(egui::Id::unique("review")).show(ctx, |ui| {
            ui.set_width(460.0);
            ui.label(RichText::new("Apply these changes?").strong().size(16.0));
            ui.add_space(6.0);

            for change in &changes {
                ui.horizontal_top(|ui| {
                    ui.label("•");
                    ui.add(egui::Label::new(change.description(self.platform)).wrap());
                });
            }

            ui.add_space(10.0);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add(
                        egui::Button::new(RichText::new("Apply").strong())
                            .min_size(egui::vec2(80.0, 26.0)),
                    )
                    .clicked()
                {
                    decision = Some(true);
                }

                if ui
                    .add(egui::Button::new("Cancel").min_size(egui::vec2(80.0, 26.0)))
                    .clicked()
                {
                    decision = Some(false);
                }
            });
        });

        match decision {
            Some(true) => {
                self.is_review_open = false;
                self.apply(changes);
            }
            Some(false) => self.is_review_open = false,
            None => {}
        }
    }
}

impl eframe::App for SetupWindow {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.receive();

        if self.setup_events.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let margin = egui::Margin::symmetric(16, 10);
        let fill = ui.visuals().panel_fill;

        egui::Panel::top("header")
            .frame(egui::Frame::new().fill(fill).inner_margin(margin))
            .show(ui, Self::show_header);
        egui::Panel::bottom("actions")
            .frame(egui::Frame::new().fill(fill).inner_margin(margin))
            .show(ui, |ui| self.show_actions(ui));
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(fill)
                    .inner_margin(egui::Margin::symmetric(16, 4)),
            )
            .show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    self.show_status(ui);
                    ui.add_enabled_ui(self.setup_events.is_none(), |ui| self.show_choices(ui));
                });
            });

        if self.is_review_open {
            let ctx = ui.ctx().clone();

            self.show_review(&ctx);
        }
    }
}

impl SetupChange {
    fn description(&self, platform: Platform) -> String {
        match self {
            Self::InstallBepInEx(major) => {
                let release = BepInExRelease::of(*major, platform);

                format!(
                    "Download BepInEx {} from {} and install it",
                    release.version, release.url
                )
            }
            Self::InstallBepInExZip(zip) => format!("Install BepInEx from {}", zip.display()),
            Self::RemoveBepInEx => "Remove BepInEx (plugins and settings are kept)".to_owned(),
            Self::InstallHookRelease(_) => format!(
                "Download the message hook of {} from github.com/{} and install it",
                MessageHook::release_tag(),
                MessageHook::DEFAULT_REPOSITORY
            ),
            Self::InstallHookFile(_, file) => {
                format!("Install the message hook from {}", file.display())
            }
            Self::RemoveHook => "Remove the message hook".to_owned(),
            Self::SetPort(port) => format!("Set the port to {port}"),
            Self::RemoveSteamApi => "Remove the duplicate libsteam_api.dylib".to_owned(),
        }
    }

    fn run(
        &self,
        setup: &Setup<'_>,
        downloader: &Downloader,
        sender: &Sender<SetupEvent>,
    ) -> SetupResult<()> {
        let mut progress = |text: String| {
            let _ = sender.send(SetupEvent::Progress(text));
        };

        match self {
            Self::InstallBepInEx(major) => {
                setup.install_bepinex(&BepInExSource::Download(*major), &mut progress)?;
            }
            Self::InstallBepInExZip(zip) => {
                setup.install_bepinex(&BepInExSource::Archive(zip.clone()), &mut progress)?;
            }
            Self::RemoveBepInEx => setup.remove_bepinex(&mut progress)?,
            Self::InstallHookRelease(major) => {
                let release = HookRelease::find(
                    downloader,
                    MessageHook::DEFAULT_REPOSITORY,
                    Some(&MessageHook::release_tag()),
                )?;

                setup.install_hook(*major, &HookSource::Release(release), &mut progress)?;
            }
            Self::InstallHookFile(major, file) => {
                setup.install_hook(*major, &HookSource::Assembly(file.clone()), &mut progress)?;
            }
            Self::RemoveHook => setup.remove_hook(&mut progress)?,
            Self::SetPort(port) => setup.configure_port(u32::from(*port), &mut progress)?,
            Self::RemoveSteamApi => setup.remove_duplicate_steam_api(&mut progress)?,
        }

        Ok(())
    }
}
