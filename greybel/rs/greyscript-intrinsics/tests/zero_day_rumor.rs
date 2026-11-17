use greyhack_world::{PlayerOptions, World};

use crate::support::scripts::run_on_host;

#[test]
fn should_expose_the_zero_day_admin_operations() {
    let world =
        greybel_test_support::future::block_on(World::new(&PlayerOptions::default())).unwrap();
    let (output, error, _) = run_on_host(
        r#"
admin = get_greybel_util()
admin.set_date("01/Feb/2024 - 00:00")
print admin.debug_offer()
print admin.debug_offer("npc", "pw", 1000)
admin.create_mail_account("neo@test.com", "matrix1")
print admin.get_mail_account("neo@test.com")
print admin.create_zero_day_chain()
print admin.create_procedural_mission(42)
rumor = admin.request_zero_day_mission()
print rumor.public_ip
print rumor.lan_ip
print admin.visit_zero_day_hack_shop(rumor.public_ip)
print admin.network(get_router.public_ip).unlock_debug_library(get_shell.host_computer.local_ip, "/lib/libssh.so")
"#,
        world,
        true,
    );

    assert!(error.is_none(), "{error:?}");
    assert!(output.contains("credentials are incorrect"), "{output}");
    assert!(output.contains("password_hash"), "{output}");
    assert!(output.contains('1'), "{output}");
}
