use crate::logging::init;

#[test]
fn test_init() {
    init("info", false).expect("Failed to init logger");
}
