const PROTO_DIR: &str = "../../proto";

fn main() {
    tonic_build::configure()
        .compile(
            &[
                format!("{}/scuffle/events/ingest.proto", PROTO_DIR),
                format!("{}/scuffle/backend/api.proto", PROTO_DIR),
                format!("{}/scuffle/utils/health.proto", PROTO_DIR),
            ],
            &[PROTO_DIR],
        )
        .unwrap();
}
