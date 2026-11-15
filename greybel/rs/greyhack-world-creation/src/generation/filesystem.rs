//! Builds the default file-system layout and first-install files.

use greyhack_behaviour::{
    APT_FOLDER, APT_SOURCES_PATH, Command, DeviceKind, FileData, FileKind, GUEST_USER,
    LIBRARY_FOLDER, PASSWD_PATH, PermissionSet, ROOT_USER, START_DATE, SYSTEM_LOG_PATH,
    format_apt_sources, parse_date,
};
use greyhack_world_assets::{FileLayout, FileSystemLayout, FolderLayout, Script};
use greyhack_world_core::{NodeId, User};
use greyhack_world_libraries::{ComputerDraft, FileSystemDraft, NodeDraft};

/// Wi-Fi tools installed by `ResumeFirstInstall`.
const PLAYER_COMMANDS: &[&str] = &["airmon", "aireplay", "aircrack"];
/// Programs added to player `/usr/bin`; single-player excludes `Chat.exe`.
const PLAYER_PROGRAMS: &[Command] = &[Command::Map, Command::Manual, Command::ScanLan];
/// Library added to player `/lib` by `ResumeFirstInstall`.
const PLAYER_LIBRARIES: &[&str] = &["crypto.so"];

fn perms(text: &str) -> PermissionSet {
    PermissionSet::parse(text).expect("a valid preset")
}

/// Expands `layout` into files, handling `/etc` command-file exceptions.
fn layout_file(layout: &FileLayout, folder: &str) -> FileData {
    let kind = FileKind::of_layout(layout.file_type, layout.binary || layout.executable);
    let mut data = FileData {
        kind,
        command: if layout.executable && kind == FileKind::Binary {
            Command::of_name(&layout.process)
        } else {
            None
        },
        ..FileData::default()
    };

    if layout.executable || layout.binary {
        data.size = FileData::binary_size(&layout.name, kind == FileKind::Library);
    }

    if let Some(script) = Script::find(&layout.name).filter(|_| folder != "etc") {
        data.default_content = true;
        script.source.clone_into(&mut data.content);
    } else if !layout.content.is_empty() {
        data.set_content(&layout.content);
    }

    data
}

fn add_layout_folder(fs: &mut FileSystemDraft, parent: NodeId, layout: &FolderLayout) {
    let initial = parse_date(START_DATE);
    let mut draft = NodeDraft::folder(&layout.name, ROOT_USER, perms(&layout.permissions));

    draft.timestamp = initial.saturating_sub(layout.age_minutes * 60);

    let folder = fs.add(parent, draft);

    for file in &layout.files {
        let mut draft = NodeDraft::file(
            &file.name,
            ROOT_USER,
            perms(&file.permissions),
            layout_file(file, &layout.name),
        );

        // `Archivo(..., proceso, isEjecutable, ...)` and `ConfigFile`: only executable layout files
        // retain their process command. Configured scripts, libraries, and other command binaries
        // clear it.
        draft.data_mut().expect("a layout file").command_name = if file.executable {
            file.process.clone()
        } else {
            String::new()
        };
        draft.timestamp = initial.saturating_sub(file.age_minutes * 60);
        fs.add(folder, draft);
    }

    for child in &layout.folders {
        add_layout_folder(fs, folder, child);
    }
}

fn library(name: &str, owner: &str, permissions: PermissionSet) -> NodeDraft {
    let mut node = NodeDraft::file(
        name,
        owner,
        permissions,
        FileData::library(FileData::binary_size(name, true)),
    );

    node.timestamp = parse_date(START_DATE);

    node
}

/// Adds each missing library in `names` to `/lib`.
pub(super) fn add_libraries(computer: &mut ComputerDraft, names: impl IntoIterator<Item = String>) {
    let fs = &mut computer.filesystem;
    let Some(lib) = fs.folder(LIBRARY_FOLDER, false) else {
        return;
    };

    let permissions = if fs.node(lib).permissions == PermissionSet::OPEN_FOLDER {
        PermissionSet::OPEN_FILE
    } else {
        PermissionSet::GROUP_READ_FILE
    };

    for name in names {
        if !fs.has_child(lib, &name) {
            fs.add(lib, library(&name, ROOT_USER, permissions));
        }
    }
}

/// Adds each library in `names` to player `/lib` with player ownership, matching first-install
/// files.
pub fn add_player_libraries(computer: &mut ComputerDraft, player: &str, names: &[String]) {
    let fs = &mut computer.filesystem;
    let Some(lib) = fs.folder(LIBRARY_FOLDER, false) else {
        return;
    };

    for name in names {
        if !fs.has_child(lib, name) {
            fs.add(
                lib,
                library(name, player, PermissionSet::default_for(false)),
            );
        }
    }
}

/// Commands omitted from router and smart-device `/bin` (`XmlGlobal.GenerarRouterFileSystem`).
const DEVICE_MISSING_COMMANDS: [&str; 14] = [
    "iwconfig", "iwlist", "ssh", "ftp", "reboot", "whois", "nslookup", "build", "chown", "chgrp",
    "groupadd", "groupdel", "groups", "kill",
];

/// Converts the computer layout in `fs` to a router, camera, fridge, or microwave layout
/// (`XmlGlobal.GenerarRouterFileSystem`). Removes apt, its client library, and most commands;
/// retains only the file explorer. Uses `kernel_router.so` instead of `kernel_module.so` on
/// routers.
fn trim_for_device(fs: &mut FileSystemDraft, router: bool) {
    if router && let Some(kernel) = fs.file("/lib/kernel_module.so", false) {
        "kernel_router.so".clone_into(&mut fs.node_mut(kernel).name);
    }

    if let Some(programs) = fs.folder("/usr/bin", false) {
        for id in fs.children(programs) {
            if fs.node(id).name != "FileExplorer.exe" {
                fs.remove(id);
            }
        }
    }

    let absent = [APT_FOLDER.to_owned(), "/lib/aptclient.so".to_owned()]
        .into_iter()
        .chain(
            DEVICE_MISSING_COMMANDS
                .iter()
                .map(|name| format!("/bin/{name}")),
        );

    for path in absent {
        if let Some(id) = fs.folder(&path, false).or_else(|| fs.file(&path, false)) {
            fs.remove(id);
        }
    }
}

fn inherit_open_system_log_permissions(fs: &mut FileSystemDraft) {
    if fs
        .folder("/var", false)
        .is_some_and(|var| fs.node(var).permissions == PermissionSet::OPEN_FOLDER)
        && let Some(log) = fs.file(SYSTEM_LOG_PATH, false)
    {
        // `Computer.SetGeneratedSystemLog`: the generated log inherits `/var` access permissions.
        fs.node_mut(log).permissions = PermissionSet::OPEN_FILE;
    }
}

/// Builds the default layout, user home directories, and first-install additions.
pub fn build_filesystem(computer: &mut ComputerDraft) {
    let users: Vec<String> = computer
        .users
        .iter()
        .map(|user| user.name.clone())
        .collect();
    let player = computer
        .is_player
        .then(|| users.iter().find(|user| *user != ROOT_USER).cloned())
        .flatten();
    let kind = computer.kind;
    let fs = &mut computer.filesystem;
    let root = fs.root();
    let initial = parse_date(START_DATE);
    let layout = FileSystemLayout::base();

    fs.node_mut(root).timestamp = initial.saturating_sub(layout.age_minutes * 60);
    for folder in &layout.folders {
        add_layout_folder(fs, root, folder);
    }

    inherit_open_system_log_permissions(fs);

    match kind {
        DeviceKind::Router | DeviceKind::Switch => trim_for_device(fs, true),
        DeviceKind::Camera | DeviceKind::Fridge | DeviceKind::Microwave => {
            trim_for_device(fs, false);
        }
        DeviceKind::Computer | DeviceKind::Hub | DeviceKind::Smartphone => {}
    }

    if let Some(sources) = fs.file(APT_SOURCES_PATH, false) {
        fs.node_mut(sources)
            .data_mut()
            .expect("sources.txt is a file")
            .set_content(&format_apt_sources(&[], true));
    }

    let home = fs.folder("/home", false).expect("the layout has /home");
    let root_home = fs.folder("/root", false).expect("the layout has /root");

    for user in &users {
        if user == ROOT_USER {
            User::add_home_folders(fs, root_home, user, PermissionSet::GROUP_FOLDER);
        } else {
            let mut draft = NodeDraft::folder(user, user, PermissionSet::HOME_FOLDER);

            draft.timestamp = initial;

            let folder = fs.add(home, draft);

            User::add_home_folders(fs, folder, user, PermissionSet::GROUP_FOLDER);
        }
    }

    let mut guest_draft = NodeDraft::folder(GUEST_USER, GUEST_USER, PermissionSet::OPEN_FOLDER);

    guest_draft.timestamp = initial;

    let guest = fs.add(home, guest_draft);

    User::add_home_folders(fs, guest, GUEST_USER, PermissionSet::OPEN_FOLDER);

    if let Some(player) = player {
        add_first_install(fs, &player);
    }

    computer.write_passwd(false);
    if let Some(passwd) = computer.filesystem.file(PASSWD_PATH, true) {
        computer.filesystem.node_mut(passwd).permissions = PermissionSet::GROUP_READ_FILE;
    }
}

/// Adds first-install directories, libraries, and programs for the player.
fn add_first_install(fs: &mut FileSystemDraft, player: &str) {
    let initial = parse_date(START_DATE);
    let etc = fs.folder("/etc", false).expect("the layout has /etc");
    let mut startup = NodeDraft::folder("init.d", ROOT_USER, PermissionSet::GROUP_FOLDER);

    startup.timestamp = initial;
    fs.add(etc, startup);

    let lib = fs
        .folder(LIBRARY_FOLDER, false)
        .expect("the layout has /lib");

    fs.node_mut(lib).permissions = PermissionSet::GROUP_FOLDER;
    for name in PLAYER_LIBRARIES {
        if !fs.has_child(lib, name) {
            fs.add(
                lib,
                library(name, player, PermissionSet::default_for(false)),
            );
        }
    }

    let bin = fs.folder("/bin", false).expect("the layout has /bin");

    for command in PLAYER_COMMANDS {
        let script = Script::find(command).expect("the game ships its Wi-Fi tools");
        let data = FileData {
            size: FileData::binary_size(command, false),
            ..FileData::script(script.source)
        };

        let mut node = NodeDraft::file(command, player, PermissionSet::default_for(false), data);

        node.timestamp = initial;
        fs.add(bin, node);
    }

    let usr_bin = fs
        .folder("/usr/bin", false)
        .expect("the layout has /usr/bin");

    for &program in PLAYER_PROGRAMS {
        let mut node = NodeDraft::file(
            program.name(),
            player,
            PermissionSet::default_for(false),
            FileData::application(program, FileData::binary_size(program.name(), false)),
        );

        node.timestamp = initial;
        fs.add(usr_bin, node);
    }
}

#[cfg(test)]
#[path = "tests/filesystem.rs"]
pub(crate) mod tests;
