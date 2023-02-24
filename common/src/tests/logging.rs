use crate::logging::init;

#[test]
fn test_init() {
    init("info").expect("Failed to init logger");
}

#[test]
fn test_with_bad_input() {
    init("???").expect("Failed to init logger");
}
