fn main() -> Result<(), Box<dyn std::error::Error>> {
	tonic_build::configure()
		.type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]")
		.compile(&["proto/scuffle/image_processor/service.proto"], &["proto/"])?;
	Ok(())
}
