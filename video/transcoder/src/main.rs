use std::sync::Arc;

use anyhow::Context as _;
use binary_helper::{
    bootstrap,
    global::{setup_database, setup_nats},
    grpc_health, grpc_server, impl_global_traits,
};
use common::{
    context::Context,
    global::{GlobalCtx, GlobalDb, GlobalNats},
    grpc::TlsSettings,
};
use tokio::select;
use video_transcoder::config::TranscoderConfig;

#[derive(Debug, Clone, Default, serde::Deserialize, config::Config)]
#[serde(default)]
struct ExtConfig {
    /// The Transcoder configuration.
    transcoder: TranscoderConfig,
}

impl binary_helper::config::ConfigExtention for ExtConfig {
    const APP_NAME: &'static str = "video-transcoder";
}

type AppConfig = binary_helper::config::AppConfig<ExtConfig>;

struct GlobalState {
    ctx: Context,
    config: AppConfig,
    nats: async_nats::Client,
    jetstream: async_nats::jetstream::Context,
    db: Arc<sqlx::PgPool>,
    metadata_store: async_nats::jetstream::kv::Store,
    media_store: async_nats::jetstream::object_store::ObjectStore,
    ingest_tls: Option<common::grpc::TlsSettings>,
}

impl_global_traits!(GlobalState);

impl common::global::GlobalConfigProvider<TranscoderConfig> for GlobalState {
    #[inline(always)]
    fn provide_config(&self) -> &TranscoderConfig {
        &self.config.extra.transcoder
    }
}

impl video_transcoder::global::TranscoderState for GlobalState {
    #[inline(always)]
    fn metadata_store(&self) -> &async_nats::jetstream::kv::Store {
        &self.metadata_store
    }

    #[inline(always)]
    fn media_store(&self) -> &async_nats::jetstream::object_store::ObjectStore {
        &self.media_store
    }

    #[inline(always)]
    fn ingest_tls(&self) -> Option<common::grpc::TlsSettings> {
        self.ingest_tls.clone()
    }
}

#[async_trait::async_trait]
impl binary_helper::Global<AppConfig> for GlobalState {
    async fn new(ctx: Context, config: AppConfig) -> anyhow::Result<Self> {
        let (nats, jetstream) = setup_nats(&config.name, &config.nats).await?;
        let db = setup_database(&config.database).await?;

        let metadata_store = jetstream
            .get_key_value(&config.extra.transcoder.metadata_kv_store)
            .await
            .context("failed to get metadata store")?;
        let media_store = jetstream
            .get_object_store(&config.extra.transcoder.media_ob_store)
            .await
            .context("failed to get media store")?;

        let ingest_tls = if let Some(tls) = &config.extra.transcoder.ingest_tls {
            let cert = tokio::fs::read(&tls.cert)
                .await
                .context("failed to read ingest tls cert")?;
            let key = tokio::fs::read(&tls.key)
                .await
                .context("failed to read ingest tls key")?;
            let ca_cert = tokio::fs::read(&tls.ca_cert)
                .await
                .context("failed to read ingest tls ca")?;

            Some(TlsSettings {
                domain: tls.domain.clone(),
                ca_cert: tonic::transport::Certificate::from_pem(ca_cert),
                identity: tonic::transport::Identity::from_pem(cert, key),
            })
        } else {
            None
        };

        Ok(Self {
            ctx,
            config,
            nats,
            jetstream,
            db,
            metadata_store,
            media_store,
            ingest_tls,
        })
    }
}

#[tokio::main]
pub async fn main() {
    if let Err(err) = bootstrap::<AppConfig, GlobalState, _>(|global| async move {
        let grpc_future = {
            let mut server = grpc_server(&global.config.grpc)
                .await
                .context("failed to create grpc server")?;
            let router = server.add_service(grpc_health::HealthServer::new(
                &global,
                |global, _| async move {
                    !global.db().is_closed()
                        && global.nats().connection_state()
                            == async_nats::connection::State::Connected
                },
            ));

            let router = video_transcoder::grpc::add_routes(&global, router);

            router.serve_with_shutdown(global.config.grpc.bind_address, async {
                global.ctx().done().await;
            })
        };

        let transcoder_future = video_transcoder::transcoder::run(global.clone());

        select! {
            r = grpc_future => r.context("grpc server stopped unexpectedly")?,
            r = transcoder_future => r.context("transcoder server stopped unexpectedly")?,
        }

        Ok(())
    })
    .await
    {
        tracing::error!("{:#}", err);
        std::process::exit(1);
    }
}
