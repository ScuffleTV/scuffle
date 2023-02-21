use super::*;

#[test]
fn test_init() {
    init("info").unwrap();
}

#[test]
fn test_with_bad_input() {
    init("???").unwrap();
}
