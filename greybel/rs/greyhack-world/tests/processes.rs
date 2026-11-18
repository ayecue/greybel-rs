use greyhack_behaviour::{FileData, PermissionSet};
use greyhack_world::{PlayerOptions, World};
use greyhack_world_assets::Script;
use greyhack_world_core::NodeDraft;
use greyhack_world_shell::{Launch, LaunchFailure};

#[tokio::test]
async fn should_refuse_gui_programs_in_the_safe_terminal() {
    let world = World::new(&PlayerOptions::default()).await.unwrap();

    assert_eq!(
        world
            .modules
            .shell
            .launches
            .launch_program(world.player(), "root", "/usr/bin/Notepad.exe", &[], 0, true,)
            .await
            .unwrap(),
        Launch::Refusal(LaunchFailure::GuiUnavailable)
    );
}

#[tokio::test]
async fn should_preserve_protected_system_processes_when_the_player_reboots() {
    let world = World::new(&PlayerOptions::default()).await.unwrap();
    let player = world.handle.player();
    let system_processes: Vec<_> = world
        .modules
        .core
        .computers
        .processes
        .processes(player)
        .await
        .unwrap()
        .into_iter()
        .filter(|process| {
            process.protected && (process.name == "kernel_task" || process.name == "Xorg")
        })
        .collect();

    world
        .modules
        .core
        .computers
        .processes
        .spawn_process(player, "root", "user_process", None)
        .await
        .unwrap();
    world
        .modules
        .core
        .computers
        .processes
        .reboot_computer(player, 180)
        .await
        .unwrap();

    assert_eq!(
        world
            .modules
            .core
            .computers
            .processes
            .processes(player)
            .await
            .unwrap(),
        system_processes
    );
}

#[tokio::test]
async fn should_list_a_session_for_each_logged_in_user_before_the_processes() {
    let world = World::new(&PlayerOptions::default()).await.unwrap();
    let player = world.handle.player();
    let npc = world
        .modules
        .npcs
        .npcs
        .add_npc(player, "root", "Ann", u64::MAX)
        .await
        .unwrap()
        .unwrap();
    let mut person = world.modules.npcs.npcs.person(npc).await.unwrap().unwrap();

    person.surname = "straße".to_owned();
    world.modules.npcs.npcs.save_person(&person).await.unwrap();
    world
        .modules
        .core
        .computers
        .users
        .open_session(player, "root")
        .await
        .unwrap();
    world
        .modules
        .core
        .computers
        .users
        .open_session(player, "root")
        .await
        .unwrap();

    let table = world
        .modules
        .core
        .computers
        .processes
        .process_table(player)
        .await
        .unwrap();

    // The game hashes "straße" to -950453660 (mono-string's fixtures), and `new Random(-950453660).Next(12000,
    // 20000)` is 17391.
    let session = &table[0];

    assert_eq!(
        (session.user.as_str(), session.pid, session.name.as_str()),
        ("root", 17391, "dsession")
    );
    assert_eq!((session.cpu, session.ram_mb), (0.0, 25.0));
    assert_eq!(
        table[1..],
        world
            .modules
            .core
            .computers
            .processes
            .processes(player)
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn should_require_execute_on_a_custom_bash() {
    let world = World::new(&PlayerOptions::default()).await.unwrap();
    let computer = world.handle.player();
    let bin = world
        .modules
        .core
        .files
        .lookup
        .folder(computer, "/bin", false)
        .await
        .unwrap()
        .unwrap();
    let mut bash = match world
        .modules
        .core
        .files
        .lookup
        .file(computer, "/bin/bash", false)
        .await
        .unwrap()
    {
        Some(bash) => bash,
        None => world
            .modules
            .libraries
            .generator
            .add_full_node(
                &bin,
                &NodeDraft::file(
                    "bash",
                    "root",
                    PermissionSet::default_for(false),
                    FileData::script("custom"),
                ),
            )
            .await
            .unwrap(),
    };

    bash.permissions = PermissionSet::parse("-rw-r--r--").unwrap();
    *bash.data_mut().unwrap() = FileData::script("custom");
    world
        .modules
        .core
        .files
        .changes
        .save_node(&bash)
        .await
        .unwrap();

    let (fallback, _) = world
        .modules
        .core
        .computers
        .processes
        .bash_program(computer, "root")
        .await
        .unwrap();

    assert_eq!(fallback, Script::find("bash").unwrap().source);

    bash.permissions = PermissionSet::parse("-rw-r--r-x").unwrap();
    world
        .modules
        .core
        .files
        .changes
        .save_node(&bash)
        .await
        .unwrap();

    let (custom, _) = world
        .modules
        .core
        .computers
        .processes
        .bash_program(computer, "root")
        .await
        .unwrap();

    assert_eq!(custom, "custom");
}
