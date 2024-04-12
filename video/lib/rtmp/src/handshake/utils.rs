use std::time::SystemTime;

pub fn current_time() -> u32 {
	let duration = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH);
	match duration {
		Ok(result) => result.as_nanos() as u32,
		_ => 0,
	}
}
