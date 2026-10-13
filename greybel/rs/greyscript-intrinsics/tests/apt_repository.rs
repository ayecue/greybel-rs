use greyhack_behaviour::DEFAULT_REPOSITORY_PORT;

use crate::support::apt_repository::world_with_hackshop;
use crate::support::scripts::run_on_host;

/// `Router.PostGenComputer` gives a HackShop's repository its downloads as the catalog, and
/// `AptClientIntrinsics.install` serves them next to the official server's libraries, which the player's sources keep
/// listed.
#[test]
fn should_install_the_programs_and_libraries_a_hackshop_repository_serves() {
    let (world, address) = world_with_hackshop();
    let code = format!(
        "apt = include_lib(\"/lib/aptclient.so\"); pc = get_shell.host_computer; print apt.add_repo(\"{address}\", {DEFAULT_REPOSITORY_PORT}); print apt.update; print apt.show(\"{address}\"); print apt.search(\"scan\"); print apt.install(\"scanlib\", home_dir); file = pc.File(home_dir + \"/scanlib\"); print [file != null, file.is_binary, file.get_content]; print apt.check_upgrade(file.path); print file.set_content(\"older\"); print apt.check_upgrade(file.path); print apt.install(\"metaxploit.so\", home_dir); lib = pc.File(home_dir + \"/metaxploit.so\"); print [lib != null, lib.is_binary]; print apt.check_upgrade(lib.path); print apt.install(\"blockchain.so\", home_dir); print pc.File(home_dir + \"/blockchain.so\") != null; print apt.check_upgrade(home_dir + \"/blockchain.so\")"
    );
    let (output, error, _) = run_on_host(&code, world, true);
    let binary = "can't open /home/test/scanlib. Binary file.";

    assert_eq!(error, None);
    assert!(
        output.starts_with(&format!(
            "\nUpdating: {address}\n\n<b>AdminMonitor.exe</b>\n"
        )),
        "{output}"
    );
    assert!(output.contains("<b>librshell.so</b>\nLibrary used in the reverse"));
    assert!(output.ends_with(&format!(
        "1\n[1, 1, \"{binary}\"]\n0\n{binary}\n0\n1\n[1, 1]\n0\n1\n1\n0"
    )));
}
