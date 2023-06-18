use crate::logging::{self, init};

#[test]
fn test_init() {
    init("info", logging::Mode::Compact).expect("Failed to init logger");
}
