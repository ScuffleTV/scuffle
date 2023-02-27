const PROTO_DIR: &str = "../../proto";

fn main() {
    let mut config = prost_build::Config::new();

    config.protoc_arg("--experimental_allow_proto3_optional");
    config.bytes(["."]);

    tonic_build::configure()
        .compile_with_config(
            config,
            &[
                format!("{}/scuffle/events/ingest.proto", PROTO_DIR),
                format!("{}/scuffle/events/api.proto", PROTO_DIR),
                format!("{}/scuffle/backend/api.proto", PROTO_DIR),
                format!("{}/scuffle/utils/health.proto", PROTO_DIR),
            ],
            &[PROTO_DIR],
        )
        .unwrap();
}
