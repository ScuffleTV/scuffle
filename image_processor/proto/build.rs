fn main() -> Result<(), Box<dyn std::error::Error>> {
	let config = tonic_build::configure()
		.build_server(cfg!(feature = "server"))
		.build_client(cfg!(feature = "client"));

	#[cfg(feature = "serde")]
	let config = config.type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]");

	config.compile(
		&[
			"scuffle/image_processor/service.proto",
			"scuffle/image_processor/types.proto",
			"scuffle/image_processor/events.proto",
		],
		&["./"],
	)?;

	Ok(())
}
