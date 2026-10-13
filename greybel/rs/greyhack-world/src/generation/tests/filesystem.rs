use super::*;

#[test]
fn should_inherit_open_folder_permissions_for_generated_content() {
    let mut computer = ComputerDraft::new("test", "192.168.0.2");

    build_filesystem(&mut computer);

    let fs = &mut computer.filesystem;
    let lib = fs.folder("/lib", false).unwrap();

    fs.node_mut(lib).permissions = perms("drwxrwxrwx");

    add_libraries(&mut computer, ["test.so".to_owned()]);

    let fs = &computer.filesystem;
    let library = fs.file("/lib/test.so", false).unwrap();

    assert_eq!(fs.node(library).permissions.as_str(), "-rwxrwxrwx");

    let mut computer = ComputerDraft::new("test", "192.168.0.2");

    build_filesystem(&mut computer);

    let fs = &mut computer.filesystem;
    let var = fs.folder("/var", false).unwrap();

    fs.node_mut(var).permissions = perms("drwxrwxrwx");

    inherit_open_system_log_permissions(fs);

    let log = fs.file("/var/system.log", false).unwrap();

    assert_eq!(fs.node(log).permissions.as_str(), "-rwxrwxrwx");
}
