use crate::logging::{
	init, {self},
};

#[test]
fn test_init() {
	init("info", logging::Mode::Compact).expect("Failed to init logger");
}
