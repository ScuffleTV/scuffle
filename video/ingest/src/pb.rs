pub mod scuffle {
    pub mod backend {
        tonic::include_proto!("scuffle.backend");
    }

    pub mod types {
        tonic::include_proto!("scuffle.types");
    }

    pub mod video {
        tonic::include_proto!("scuffle.video");
    }

    pub mod events {
        tonic::include_proto!("scuffle.events");
    }
}

pub mod health {
    tonic::include_proto!("grpc.health.v1");
}
