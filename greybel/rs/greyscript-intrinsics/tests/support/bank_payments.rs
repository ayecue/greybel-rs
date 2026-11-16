//! What the bank payment tests of the intrinsics share.

/// Hires an administrator, removes him, and tries again.
pub const SCRIPT: &str = r#"
admin = get_greybel_util()
network = admin.network(get_router.public_ip)
print network.hire_admin
print network.fire_admin
print network.fire_admin
print network.hire_admin
"#;
