use crate::database::global_role::Permission;

#[test]
fn test_has_permission_admin() {
    let p = Permission::Admin | Permission::GoLive;

    // Admin has all permissions
    assert!(p.has_permission(
        Permission::Admin
            | Permission::GoLive
            | Permission::StreamRecording
            | Permission::StreamTranscoding
    ));
}

#[test]
fn test_has_permission_go_live() {
    let p = Permission::GoLive;

    // GoLive has GoLive permission
    assert!(p.has_permission(Permission::GoLive));

    // GoLive does not have Admin permission
    assert!(!p.has_permission(Permission::Admin));
}

#[test]
fn test_has_permission_default() {
    let p = Permission::default();

    // default has no permissions
    assert!(p.is_none());
}
