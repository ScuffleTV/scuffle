use std::sync::Arc;
use std::time::Duration;

use anyhow::Context as _;
use async_nats::jetstream::stream::{self, RetentionPolicy};
use binary_helper::global::{setup_database, setup_nats, setup_redis};
use binary_helper::impl_global_traits;
use common::config::RedisConfig;
use common::context::Context;
use common::dataloader::DataLoader;
use common::logging;
use common::prelude::FutureTimeout;
use futures_util::stream::BoxStream;
use pb::scuffle::video::v1::types::{access_token_scope, AccessTokenScope};
pub use pb::scuffle::video::v1::*;
use ulid::Ulid;
use video_api::api::ApiRequest;
use video_api::config::ApiConfig;
use video_api::dataloaders;
use video_common::database::AccessToken;

use super::request::impl_request;
use crate::cli::display::{DeleteResponse, DeleteResponseFailed, TagResponse};
pub use crate::invoker::request::*;

pub struct DirectBackend {
	access_token: Option<AccessToken>,
	global: Arc<GlobalState>,
}

impl DirectBackend {
	pub async fn new(context: Context, config_path: Option<String>, organization_id: Option<Ulid>) -> anyhow::Result<Self> {
		let global = Arc::new(GlobalState::new(context, config_path).await?);

		logging::init(&global.config.logging.level, global.config.logging.mode).expect("failed to init logging");

		let access_token = if let Some(organization_id) = organization_id {
			sqlx::query("SELECT * FROM organizations WHERE id = $1")
				.bind(common::database::Ulid(organization_id))
				.fetch_optional(global.db.as_ref())
				.await
				.context("failed to fetch the organization from the database")?
				.ok_or_else(|| {
					anyhow::anyhow!(
						"the organization specified ({organization_id}) does not exist, consider creating an organization first"
					)
				})?;

			Some(AccessToken {
				organization_id: organization_id.into(),
				scopes: vec![
					AccessTokenScope {
						permission: vec![access_token_scope::Permission::Admin as i32],
						resource: None,
					}
					.into(),
				],
				..Default::default()
			})
		} else {
			None
		};

		Ok(Self { access_token, global })
	}

	fn access_token(&self) -> anyhow::Result<&AccessToken> {
		self.access_token
			.as_ref()
			.ok_or_else(|| anyhow::anyhow!("this request requires an organization id to be specified"))
	}

	async fn generic_response<T, R>(&self, req: T) -> anyhow::Result<R>
	where
		tonic::Request<T>: ApiRequest<R>,
	{
		Ok(tonic::Request::new(req)
			.process(&self.global, self.access_token()?)
			.await
			.with_context(|| format!("failed to process {}", std::any::type_name::<T>()))?
			.into_inner())
	}

	async fn create_organization(&self, req: OrganizationCreateRequest) -> anyhow::Result<Organization> {
		let org: video_common::database::Organization =
			sqlx::query_as("INSERT INTO organizations (id, name, tags) VALUES ($1, $2, $3) RETURNING *")
				.bind(common::database::Ulid(Ulid::new()))
				.bind(req.name)
				.bind(sqlx::types::Json(req.tags))
				.fetch_one(self.global.db.as_ref())
				.await
				.context("failed to create the organization")?;

		Ok(Organization {
			id: org.id.0,
			name: org.name,
			tags: org.tags,
			updated_at: org.updated_at,
		})
	}

	async fn delete_organization(&self, req: OrganizationDeleteRequest) -> anyhow::Result<DeleteResponse> {
		let mut ids = Vec::new();
		let mut failed = Vec::new();

		for id in req.ids {
			let result = sqlx::query("DELETE FROM organizations WHERE id = $1")
				.bind(common::database::Ulid(id))
				.execute(self.global.db.as_ref())
				.await;

			match result {
				Ok(org) => {
					if org.rows_affected() == 0 {
						failed.push(DeleteResponseFailed {
							id,
							error: "the organization does not exist".into(),
						})
					} else {
						ids.push(id)
					}
				}
				Err(err) => failed.push(DeleteResponseFailed {
					id,
					error: err.to_string(),
				}),
			}
		}

		Ok(DeleteResponse { ids, failed })
	}

	async fn get_organization(&self, req: OrganizationGetRequest) -> anyhow::Result<Vec<Organization>> {
		let mut qb = sqlx::query_builder::QueryBuilder::default();

		qb.push("SELECT * FROM organizations");

		let search_options = req.search_options.unwrap_or_default();

		let mut first = true;

		if let Some(tags) = search_options.tags {
			qb.push(" WHERE ");
			first = false;
			qb.push("tags @> ");
			qb.push_bind(sqlx::types::Json(tags.tags));
		}

		if let Some(after_id) = search_options.after_id {
			if first {
				qb.push(" WHERE ");
			} else {
				qb.push(" AND ");
			}

			qb.push("id > ");
			qb.push_bind(common::database::Ulid(after_id.into_ulid()));
		}

		if search_options.reverse {
			qb.push(" ORDER BY id DESC");
		} else {
			qb.push(" ORDER BY id ASC");
		}

		qb.push(" LIMIT ");
		qb.push_bind(search_options.limit.max(100).min(1000));

		let orgs: Vec<video_common::database::Organization> = qb
			.build_query_as()
			.fetch_all(self.global.db.as_ref())
			.await
			.context("failed to fetch organizations")?;

		Ok(orgs
			.into_iter()
			.map(|org| Organization {
				id: org.id.0,
				name: org.name,
				tags: org.tags,
				updated_at: org.updated_at,
			})
			.collect())
	}

	async fn modify_organization(&self, req: OrganizationModifyRequest) -> anyhow::Result<Organization> {
		let mut qb = sqlx::query_builder::QueryBuilder::default();

		qb.push("UPDATE organizations SET ");

		let mut first = true;

		if let Some(name) = req.name {
			first = false;
			qb.push("name = ");
			qb.push_bind(name);
		}

		if let Some(tags) = req.tags {
			if !first {
				qb.push(", ");
			}

			qb.push("tags = ");
			qb.push_bind(sqlx::types::Json(tags));
		}

		qb.push(" WHERE id = ");
		qb.push_bind(common::database::Ulid(req.id));

		let org: video_common::database::Organization = qb
			.build_query_as()
			.fetch_one(self.global.db.as_ref())
			.await
			.context("failed to modify the organization")?;

		Ok(Organization {
			id: org.id.0,
			name: org.name,
			tags: org.tags,
			updated_at: org.updated_at,
		})
	}

	async fn tag_organization(&self, req: OrganizationTagRequest) -> anyhow::Result<TagResponse> {
		let org: video_common::database::Organization =
			sqlx::query_as("UPDATE organizations SET tags = tags || $1 WHERE id = $2 RETURNING *")
				.bind(sqlx::types::Json(req.tags))
				.bind(common::database::Ulid(req.id))
				.fetch_one(self.global.db.as_ref())
				.await
				.context("failed to tag the organization")?;

		Ok(TagResponse {
			id: org.id.0,
			tags: org.tags,
		})
	}

	async fn untag_organization(&self, req: OrganizationUntagRequest) -> anyhow::Result<TagResponse> {
		let org: video_common::database::Organization =
			sqlx::query_as("UPDATE organizations SET tags = tags - $1 WHERE id = $2 RETURNING *")
				.bind(sqlx::types::Json(req.tags))
				.bind(common::database::Ulid(req.id))
				.fetch_one(self.global.db.as_ref())
				.await
				.context("failed to untag the organization")?;

		Ok(TagResponse {
			id: org.id.0,
			tags: org.tags,
		})
	}
}

impl std::fmt::Debug for DirectBackend {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("DirectBackend")
			.field("access_token", &self.access_token)
			.finish()
	}
}

#[derive(Debug, Clone, Default, serde::Deserialize, config::Config)]
#[serde(default)]
struct ExtConfig {
	/// The API configuration.
	api: ApiConfig,

	/// The Redis configuration.
	redis: RedisConfig,
}

impl binary_helper::config::ConfigExtention for ExtConfig {
	const APP_NAME: &'static str = "video-api";
}

type AppConfig = binary_helper::config::AppConfig<ExtConfig>;

struct GlobalState {
	ctx: Context,
	nats: async_nats::Client,
	config: AppConfig,
	jetstream: async_nats::jetstream::Context,
	db: Arc<sqlx::PgPool>,
	redis: Arc<fred::clients::RedisPool>,
	access_token_loader: DataLoader<dataloaders::AccessTokenLoader>,
	recording_state_loader: DataLoader<dataloaders::RecordingStateLoader>,
	room_loader: DataLoader<dataloaders::RoomLoader>,
	events_stream: async_nats::jetstream::stream::Stream,
}

impl_global_traits!(GlobalState);

impl common::global::GlobalRedis for GlobalState {
	#[inline(always)]
	fn redis(&self) -> &Arc<fred::clients::RedisPool> {
		&self.redis
	}
}

impl common::global::GlobalConfigProvider<ApiConfig> for GlobalState {
	#[inline(always)]
	fn provide_config(&self) -> &ApiConfig {
		&self.config.extra.api
	}
}

impl video_api::global::ApiState for GlobalState {
	#[inline(always)]
	fn access_token_loader(&self) -> &DataLoader<dataloaders::AccessTokenLoader> {
		&self.access_token_loader
	}

	#[inline(always)]
	fn recording_state_loader(&self) -> &DataLoader<dataloaders::RecordingStateLoader> {
		&self.recording_state_loader
	}

	#[inline(always)]
	fn room_loader(&self) -> &DataLoader<dataloaders::RoomLoader> {
		&self.room_loader
	}

	#[inline(always)]
	fn events_stream(&self) -> &async_nats::jetstream::stream::Stream {
		&self.events_stream
	}
}

impl GlobalState {
	async fn new(ctx: Context, config_path: Option<String>) -> anyhow::Result<Self> {
		let (config, _) = common::config::parse::<AppConfig>(false, config_path).context("failed to parse config")?;

		let (nats, jetstream) = setup_nats(&config.name, &config.nats)
			.timeout(Duration::from_secs(2))
			.await
			.context("failed to connect to nats")?
			.context("failed to connect to nats")?;
		let db = setup_database(&config.database)
			.timeout(Duration::from_secs(2))
			.await
			.context("failed to connect to database")?
			.context("failed to connect to database")?;
		let redis = setup_redis(&config.extra.redis)
			.timeout(Duration::from_secs(2))
			.await
			.context("failed to connect to redis")?
			.context("failed to connect to redis")?;

		let access_token_loader = dataloaders::AccessTokenLoader::new(db.clone());
		let recording_state_loader = dataloaders::RecordingStateLoader::new(db.clone());
		let room_loader = dataloaders::RoomLoader::new(db.clone());

		common::ratelimiter::load_rate_limiter_script(&*redis)
			.await
			.context("failed to load rate limiter script")?;

		let events_stream = jetstream
			.get_or_create_stream(stream::Config {
				name: config.extra.api.events.stream_name.clone(),
				subjects: vec![format!("{}.>", config.extra.api.events.stream_name)],
				retention: RetentionPolicy::WorkQueue,
				max_age: config.extra.api.events.nats_stream_message_max_age,
				..Default::default()
			})
			.await
			.context("failed to create event stream")?;

		Ok(Self {
			ctx,
			config,
			nats,
			jetstream,
			db,
			redis,
			access_token_loader,
			recording_state_loader,
			room_loader,
			events_stream,
		})
	}
}

impl_request!(
	DirectBackend;

	|self, req: AccessTokenCreateRequest| -> AccessTokenCreateResponse {
		self.generic_response(req).await
	},
	|self, req: AccessTokenDeleteRequest| -> AccessTokenDeleteResponse {
		self.generic_response(req).await
	},
	|self, req: AccessTokenGetRequest| -> AccessTokenGetResponse {
		self.generic_response(req).await
	},
	|self, req: AccessTokenTagRequest| -> AccessTokenTagResponse {
		self.generic_response(req).await
	},
	|self, req: AccessTokenUntagRequest| -> AccessTokenUntagResponse {
		self.generic_response(req).await
	},

	|self, req: EventsFetchRequest| -> BoxStream<'static, tonic::Result<EventsFetchResponse>> {
		self.generic_response(req).await
	},
	|self, req: EventsAckRequest| -> EventsAckResponse {
		self.generic_response(req).await
	},

	|self, req: OrganizationCreateRequest| -> Organization {
		self.create_organization(req).await
	},
	|self, req: OrganizationDeleteRequest| -> DeleteResponse {
		self.delete_organization(req).await
	},
	|self, req: OrganizationGetRequest| -> Vec<Organization> {
		self.get_organization(req).await
	},
	|self, req: OrganizationModifyRequest| -> Organization {
		self.modify_organization(req).await
	},
	|self, req: OrganizationTagRequest| -> TagResponse {
		self.tag_organization(req).await
	},
	|self, req: OrganizationUntagRequest| -> TagResponse {
		self.untag_organization(req).await
	},

	|self, req: PlaybackKeyPairCreateRequest| -> PlaybackKeyPairCreateResponse {
		self.generic_response(req).await
	},
	|self, req: PlaybackKeyPairDeleteRequest| -> PlaybackKeyPairDeleteResponse {
		self.generic_response(req).await
	},
	|self, req: PlaybackKeyPairGetRequest| -> PlaybackKeyPairGetResponse {
		self.generic_response(req).await
	},
	|self, req: PlaybackKeyPairTagRequest| -> PlaybackKeyPairTagResponse {
		self.generic_response(req).await
	},
	|self, req: PlaybackKeyPairUntagRequest| -> PlaybackKeyPairUntagResponse {
		self.generic_response(req).await
	},
	|self, req: PlaybackKeyPairModifyRequest| -> PlaybackKeyPairModifyResponse {
		self.generic_response(req).await
	},

	|self, req: PlaybackSessionCountRequest| -> PlaybackSessionCountResponse {
		self.generic_response(req).await
	},
	|self, req: PlaybackSessionGetRequest| -> PlaybackSessionGetResponse {
		self.generic_response(req).await
	},
	|self, req: PlaybackSessionRevokeRequest| -> PlaybackSessionRevokeResponse {
		self.generic_response(req).await
	},

	|self, req: RecordingDeleteRequest| -> RecordingDeleteResponse {
		self.generic_response(req).await
	},
	|self, req: RecordingGetRequest| -> RecordingGetResponse {
		self.generic_response(req).await
	},
	|self, req: RecordingModifyRequest| -> RecordingModifyResponse {
		self.generic_response(req).await
	},
	|self, req: RecordingTagRequest| -> RecordingTagResponse {
		self.generic_response(req).await
	},
	|self, req: RecordingUntagRequest| -> RecordingUntagResponse {
		self.generic_response(req).await
	},

	|self, req: RecordingConfigCreateRequest| -> RecordingConfigCreateResponse {
		self.generic_response(req).await
	},
	|self, req: RecordingConfigDeleteRequest| -> RecordingConfigDeleteResponse {
		self.generic_response(req).await
	},
	|self, req: RecordingConfigGetRequest| -> RecordingConfigGetResponse {
		self.generic_response(req).await
	},
	|self, req: RecordingConfigModifyRequest| -> RecordingConfigModifyResponse {
		self.generic_response(req).await
	},
	|self, req: RecordingConfigTagRequest| -> RecordingConfigTagResponse {
		self.generic_response(req).await
	},
	|self, req: RecordingConfigUntagRequest| -> RecordingConfigUntagResponse {
		self.generic_response(req).await
	},

	|self, req: RoomCreateRequest| -> RoomCreateResponse {
		self.generic_response(req).await
	},
	|self, req: RoomDeleteRequest| -> RoomDeleteResponse {
		self.generic_response(req).await
	},
	|self, req: RoomGetRequest| -> RoomGetResponse {
		self.generic_response(req).await
	},
	|self, req: RoomModifyRequest| -> RoomModifyResponse {
		self.generic_response(req).await
	},
	|self, req: RoomTagRequest| -> RoomTagResponse {
		self.generic_response(req).await
	},
	|self, req: RoomUntagRequest| -> RoomUntagResponse {
		self.generic_response(req).await
	},
	|self, req: RoomDisconnectRequest| -> RoomDisconnectResponse {
		self.generic_response(req).await
	},
	|self, req: RoomResetKeyRequest| -> RoomResetKeyResponse {
		self.generic_response(req).await
	},

	|self, req: S3BucketCreateRequest| -> S3BucketCreateResponse {
		self.generic_response(req).await
	},
	|self, req: S3BucketDeleteRequest| -> S3BucketDeleteResponse {
		self.generic_response(req).await
	},
	|self, req: S3BucketGetRequest| -> S3BucketGetResponse {
		self.generic_response(req).await
	},
	|self, req: S3BucketModifyRequest| -> S3BucketModifyResponse {
		self.generic_response(req).await
	},
	|self, req: S3BucketTagRequest| -> S3BucketTagResponse {
		self.generic_response(req).await
	},
	|self, req: S3BucketUntagRequest| -> S3BucketUntagResponse {
		self.generic_response(req).await
	},

	|self, req: TranscodingConfigCreateRequest| -> TranscodingConfigCreateResponse {
		self.generic_response(req).await
	},
	|self, req: TranscodingConfigDeleteRequest| -> TranscodingConfigDeleteResponse {
		self.generic_response(req).await
	},
	|self, req: TranscodingConfigGetRequest| -> TranscodingConfigGetResponse {
		self.generic_response(req).await
	},
	|self, req: TranscodingConfigModifyRequest| -> TranscodingConfigModifyResponse {
		self.generic_response(req).await
	},
	|self, req: TranscodingConfigTagRequest| -> TranscodingConfigTagResponse {
		self.generic_response(req).await
	},
	|self, req: TranscodingConfigUntagRequest| -> TranscodingConfigUntagResponse {
		self.generic_response(req).await
	},
);
