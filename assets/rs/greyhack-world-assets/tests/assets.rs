use greyhack_world_assets::{FileSystemLayout, GameTextTable, MissionList, NameList, Script};

#[test]
fn should_hold_every_command_the_game_registers() {
    let scripts = Script::all();

    let commands: Vec<&str> = scripts.iter().map(|script| script.command).collect();

    assert_eq!(commands.len(), 62);
}

#[test]
fn should_hold_commands_without_the_command_header() {
    let headerless = ["bash", "debug_net", "masterkey", "ssh"];

    let found: Vec<bool> = headerless
        .iter()
        .map(|command| {
            Script::find(command).is_some_and(|script| !script.source.starts_with("//command: "))
        })
        .collect();

    assert_eq!(found, [true; 4]);
}

#[test]
fn should_have_a_script_for_every_script_file_of_the_base_layout() {
    let bin = FileSystemLayout::base()
        .folder("bin")
        .expect("the layout has /bin");

    let missing: Vec<&str> = bin
        .files
        .iter()
        .filter(|file| file.file_type == 1 && Script::find(&file.name).is_none())
        .map(|file| file.name.as_str())
        .collect();

    assert!(missing.is_empty(), "{missing:?}");
}

#[test]
fn should_read_the_base_file_system_layout() {
    let layout = FileSystemLayout::base();

    let folders: Vec<&str> = layout
        .folders
        .iter()
        .map(|folder| folder.name.as_str())
        .collect();
    let ls = layout.folder("bin").and_then(|bin| bin.file("ls"));

    assert_eq!(
        folders,
        [
            "etc", "lib", "sys", "root", "home", "var", "bin", "usr", "boot"
        ]
    );
    assert!(ls.is_some_and(|ls| ls.permissions == "-rwxr-xr-x"
        && ls.process == "ls"
        && ls.executable
        && ls.binary
        && ls.file_type == 1));
}

#[test]
fn should_keep_the_games_header_of_httpd_naming_sshd() {
    let command = "httpd";
    let script = Script::find(command);

    assert!(script.is_some_and(|script| script.source.starts_with("//command: sshd\n")));
}

#[test]
fn should_find_a_script_by_its_command() {
    let command = "ls";
    let script = Script::find(command);

    assert!(script.is_some_and(|script| script.source.contains("command_info(\"ls_usage\")")));
}

#[test]
fn should_split_name_lists_at_line_feeds_as_the_game_does() {
    let lists = [NameList::Corporations, NameList::MaleNames];
    let entries = lists.map(|list| {
        (
            list.entries().len(),
            list.entries().last().is_some_and(|entry| entry.is_empty()),
        )
    });

    assert_eq!(entries, [(1159, true), (3900, false)]);
}

#[test]
fn should_hold_every_name_list() {
    let lists = NameList::ALL;
    let sizes: Vec<usize> = lists.iter().map(|list| list.entries().len()).collect();

    assert_eq!(sizes, [3900, 4951, 16208, 9998, 1159, 1558, 736, 438]);
}

#[test]
fn should_answer_the_usage_texts_the_scripts_ask_for_except_three_the_game_lacks() {
    let texts = GameTextTable::english();
    let missing: std::collections::BTreeSet<String> = Script::all()
        .iter()
        .flat_map(|script| script.source.split("command_info(\"").skip(1))
        .filter_map(|others| others.split('"').next())
        .map(str::to_uppercase)
        .filter(|key| texts.get(key).is_none())
        .collect();
    let missing: Vec<String> = missing.into_iter().collect();

    assert_eq!(
        (texts.locale(), texts.len(), missing),
        (
            "en",
            1641,
            vec![
                "CD_USAGE".to_owned(),
                "REPOD_USAGE".to_owned(),
                "RSHELLD_USAGE".to_owned()
            ]
        )
    );
}

#[test]
fn should_hold_the_reverse_shell_social_engineering_full_text() {
    assert!(GameTextTable::english().get("ING_RSHELL_FULL").is_some());
}

#[test]
fn should_hold_the_reserved_address_ranges() {
    let ranges = greyhack_world_assets::reserved_address_ranges();

    assert_eq!(ranges.len(), 15);
    assert_eq!(
        ranges[2],
        ("100.64.0.0".to_owned(), "100.127.255.255".to_owned())
    );
}

#[test]
fn should_hold_the_world_items_random_content_is_drawn_from() {
    let items = greyhack_world_assets::WorldItemTable::get();
    let texts = GameTextTable::english();
    let missing: Vec<&str> = items
        .texts
        .iter()
        .chain(&items.neurobox_texts)
        .map(|item| item.id.as_str())
        .filter(|id| texts.get(id).is_none())
        .collect();

    assert_eq!(
        (
            items.images.len(),
            items.texts.len(),
            items.neurobox_texts.len(),
            missing
        ),
        (27, 17, 9, Vec::new())
    );
    assert!(items.texts[0].only_file && !items.texts[0].only_mail);
    assert!(items.images.iter().all(|image| !image.name.is_empty()));
}

#[test]
fn should_hold_every_part_exploits_are_made_of() {
    use greyhack_world_assets::ExploitPart;

    for part in ExploitPart::ALL {
        assert!(!part.source().is_empty(), "{}", part.name());
    }

    let bases = [
        ExploitPart::BaseScript,
        ExploitPart::BaseScriptRemote,
        ExploitPart::BaseScriptRouter,
        ExploitPart::BaseScriptRouterShell,
    ];

    assert!(bases[0].source().contains("<PATH_LIB>"));
    assert!(bases[1].source().contains("net_session.dump_lib"));
    assert!(bases[2].source().contains("<MEM_ZONE>"));
    assert!(
        ExploitPart::OverflowBaseScript
            .source()
            .contains("<UNSEC_VALUE>")
    );
}

#[test]
fn should_hold_the_authored_mission_lists_of_the_game() {
    let counts: Vec<usize> = MissionList::ALL
        .iter()
        .map(|list| {
            serde_json::from_str::<Vec<serde_json::Value>>(list.text())
                .expect("a list of missions")
                .len()
        })
        .collect();

    assert_eq!(counts, [10, 3, 3, 3, 1]);
}
