use std::collections::HashMap;
use std::pin::pin;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use async_nats::jetstream::consumer::pull::{Config, MessagesErrorKind};
use async_nats::jetstream::consumer::Consumer;
use async_nats::jetstream::AckKind;
use bytes::Bytes;
use fred::interfaces::KeysInterface;
use futures_util::StreamExt;
use pb::scuffle::platform::internal::image_processor;
use pb::scuffle::platform::internal::types::{uploaded_file_metadata, ImageFormat, UploadedFileMetadata};
use postgres_from_row::tokio_postgres::IsolationLevel;
use postgres_from_row::FromRow;
use tokio::select;
use ulid::Ulid;

use crate::config::{IgDbConfig, ImageUploaderConfig};
use crate::database::{Category, FileType, UploadedFile, UploadedFileStatus};
use crate::global::ApiGlobal;

pub async fn run<G: ApiGlobal>(global: Arc<G>) -> anyhow::Result<()> {
	let config = global.config::<IgDbConfig>();
	let stream = global
		.jetstream()
		.get_or_create_stream(async_nats::jetstream::stream::Config {
			name: config.igdb_cron_subject.clone(),
			subjects: vec![config.igdb_cron_subject.clone()],
			max_messages: 1,
			discard: async_nats::jetstream::stream::DiscardPolicy::New,
			retention: async_nats::jetstream::stream::RetentionPolicy::WorkQueue,
			..Default::default()
		})
		.await
		.context("create stream")?;

	let consumer = stream
		.get_or_create_consumer(
			"igdb-cron",
			async_nats::jetstream::consumer::pull::Config {
				name: Some("igdb-cron".to_string()),
				..Default::default()
			},
		)
		.await
		.context("create consumer")?;

	select! {
		e = cron(&global, config) => e.context("cron")?,
		e = process(&global, consumer, config) => e.context("process")?,
	}
	Ok(())
}

async fn cron<G: ApiGlobal>(global: &Arc<G>, config: &IgDbConfig) -> anyhow::Result<()> {
	let mut timer = tokio::time::interval(Duration::from_secs(60));
	loop {
		timer.tick().await;
		tracing::debug!("igdb cron");
		global
			.nats()
			.publish(config.igdb_cron_subject.clone(), Bytes::new())
			.await
			.context("publish")?;
	}
}

async fn process<G: ApiGlobal>(global: &Arc<G>, consumer: Consumer<Config>, config: &IgDbConfig) -> anyhow::Result<()> {
	let mut messages = consumer.messages().await.context("messages")?;

	let duration = chrono::Duration::from_std(config.refresh_interval).context("duration")?;

	'outer: while let Some(message) = messages.next().await {
		let message = match message {
			Ok(message) => message,
			Err(err) if matches!(err.kind(), MessagesErrorKind::MissingHeartbeat) => {
				continue;
			}
			Err(err) => {
				anyhow::bail!("message: {:#}", err);
			}
		};

		let info = message.info().map_err(|e| anyhow::anyhow!("info: {e}"))?;

		let time = chrono::DateTime::<chrono::Utc>::from(std::time::SystemTime::from(info.published));
		let first_run = global
			.redis()
			.get::<Option<()>, _>("igdb_last_refresh")
			.await
			.context("redis get")?
			.is_none();

		if chrono::Utc::now() - time < duration && !first_run {
			tracing::info!("Skipping IGDB refresh");
			message
				.ack_with(AckKind::Nak(Some(Duration::from_secs(300))))
				.await
				.map_err(|e| anyhow::anyhow!("ack: {}", e))?;
			continue;
		}

		// Refresh IGDB
		tracing::info!("Refreshing IGDB");

		let refresh_igdb = refresh_igdb(global, config);
		let mut refresh_igdb = pin!(refresh_igdb);

		loop {
			select! {
				e = &mut refresh_igdb => {
					if let Err(e) = e {
						tracing::error!("igdb: {:#}", e);
						message.ack_with(AckKind::Nak(Some(Duration::from_secs(300)))).await.map_err(|e| anyhow::anyhow!("ack: {e}"))?;
						continue 'outer;
					}

					break;
				}
				_ = tokio::time::sleep(Duration::from_secs(15)) => {
					tracing::debug!("igdb: refresh ack hold");
					message.ack_with(AckKind::Progress).await.map_err(|e| anyhow::anyhow!("ack: {e}"))?;
				}
			}
		}

		global
			.redis()
			.set("igdb_last_refresh", chrono::Utc::now().to_rfc3339(), None, None, false)
			.await
			.context("redis set")?;
		message
			.ack_with(AckKind::Ack)
			.await
			.map_err(|e| anyhow::anyhow!("ack: {e}"))?;
	}

	Ok(())
}

#[derive(serde::Deserialize)]
struct CountResponse {
	count: usize,
}

#[derive(Default, serde::Deserialize)]
#[serde(default)]
struct Game {
	id: i32,
	age_ratings: Vec<AgeRating>,
	alternative_names: Vec<IdName>,
	artworks: Vec<Image>,
	cover: Option<Image>,
	genres: Vec<IdName>,
	keywords: Vec<IdName>,
	name: String,
	rating: f64,
	similar_games: Vec<i32>,
	storyline: Option<String>,
	summary: Option<String>,
	themes: Vec<IdName>,
	websites: Vec<Website>,
}

#[derive(Default, serde::Deserialize)]
#[serde(default)]
struct AgeRating {
	id: i32,
	category: i32,
	rating: i32,
}

#[derive(Default, serde::Deserialize)]
#[serde(default)]
struct IdName {
	id: i32,
	name: String,
}

#[derive(Default, serde::Deserialize)]
#[serde(default)]
struct Image {
	id: i32,
	alpha_channel: bool,
	animated: bool,
	image_id: String,
	height: i32,
	width: i32,
}

#[derive(Default, serde::Deserialize)]
#[serde(default)]
struct Website {
	id: i32,
	category: i32,
	url: String,
}

async fn refresh_igdb<G: ApiGlobal>(global: &Arc<G>, config: &IgDbConfig) -> anyhow::Result<()> {
	let access_token = global
		.redis()
		.get::<Option<String>, _>("igdb_access_token")
		.await
		.context("redis get")?;
	let access_token = if let Some(access_token) = access_token {
		access_token
	} else {
		let (access_token, ttl) = get_access_token(config).await.context("get access token")?;
		global
			.redis()
			.set(
				"igdb_access_token",
				&access_token,
				Some(fred::types::Expiration::EX((ttl / 2).max(1))),
				None,
				false,
			)
			.await
			.context("redis set")?;
		access_token
	};

	let client = reqwest::ClientBuilder::new()
		.user_agent("scuffle/0.1.0")
		.default_headers({
			let mut headers = reqwest::header::HeaderMap::new();
			headers.insert(
				reqwest::header::AUTHORIZATION,
				format!("Bearer {}", access_token).parse().unwrap(),
			);
			headers.insert(reqwest::header::ACCEPT, "application/json".parse().unwrap());
			headers.insert("Client-ID", config.client_id.parse().unwrap());
			headers
		})
		.build()
		.context("build client")?;

	// The API has a ratelimit of 4 requests per second
	// Lets start by counting the number of games there are.
	// Then we can divide that by 4 to get the number of seconds we need to wait

	let resp = client
		.post("https://api.igdb.com/v4/games/count")
		.body("where category = 0;") // Category 0 is for games (not dlc, etc)
		.send()
		.await
		.context("count request")?;

	resp.error_for_status_ref().context("count response")?;

	let resp = resp.json::<CountResponse>().await.context("count json")?;

	let total = resp.count;
	let aprox_seconds = (total as f64 / 500.0 * 1000.0 / 400.0).round() as i64;

	tracing::info!("IGDB has {total} games, a refresh will take aproximately {aprox_seconds} seconds");

	let mut timer = tokio::time::interval(Duration::from_millis(250));
	timer.tick().await;

	let mut offset = 0;

	loop {
		let resp = client.post("https://api.igdb.com/v4/games")
            .body(format!("fields name, genres.name, alternative_names.name, summary, storyline, id, age_ratings.*, artworks.*, keywords.name, rating, similar_games, url, themes.name, websites.*, cover.*; where category = 0; offset {offset}; limit 500;"))
            .send()
            .await
            .context("games request")?;

		resp.error_for_status_ref().context("games response")?;

		let resp = resp.json::<Vec<Game>>().await.context("games json")?;

		if resp.is_empty() {
			tracing::info!("igdb: done");
			break;
		}

		struct InsertItem {
			id: Ulid,
			igdb_id: i32,
			name: String,
			alternative_names: Vec<String>,
			keywords: Vec<String>,
			storyline: Option<String>,
			summary: Option<String>,
			over_18: bool,
			cover_id: Option<Ulid>,
			rating: f64,
			updated_at: chrono::DateTime<chrono::Utc>,
			artwork_ids: Vec<Ulid>,
			igdb_similar_game_ids: Vec<i32>,
			websites: Vec<String>,
		}

		let mut client = global.db().get().await.context("get db connection")?;
		let tx = client.transaction().await.context("start transaction")?;

		let image_ids = resp
			.iter()
			.flat_map(|item| {
				item.artworks
					.iter()
					.chain(item.cover.as_ref())
					.map(|x| (Ulid::new(), x.image_id.as_str()))
			})
			.collect::<Vec<_>>();

		#[derive(FromRow)]
		struct ImageId {
			image_id: String,
			uploaded_file_id: Ulid,
		}

		utils::database::query("INSERT INTO igdb_image (uploaded_file_id, image_id)")
			.push_values(&image_ids, |mut sep, item| {
				sep.push_bind(item.0);
				sep.push_bind(item.1);
			})
			.push("ON CONFLICT (image_id) DO NOTHING;")
			.build()
			.execute(&tx)
			.await
			.context("insert igdb_image")?;

		let image_ids =
			utils::database::query("SELECT image_id, uploaded_file_id FROM igdb_image WHERE image_id = ANY($1::TEXT[])")
				.bind(image_ids.iter().map(|x| x.1).collect::<Vec<_>>())
				.build_query_as::<ImageId>()
				.fetch_all(&tx)
				.await
				.context("select igdb_image")?;

		let image_ids = image_ids
			.into_iter()
			.map(|row| (row.image_id, row.uploaded_file_id))
			.collect::<HashMap<_, _>>();

		let uploaded_files = resp
			.iter()
			.flat_map(|item| {
				item.cover
					.as_ref()
					.map(|cover| UploadedFile {
						id: image_ids[&cover.image_id],
						failed: None,
						name: format!("igdb/cover/{}.jpg", cover.image_id),
						owner_id: None,
						path: format!(
							"https://images.igdb.com/igdb/image/upload/t_cover_big_2x/{}.jpg",
							cover.image_id
						),
						status: UploadedFileStatus::Unqueued,
						metadata: UploadedFileMetadata {
							metadata: Some(uploaded_file_metadata::Metadata::Image(uploaded_file_metadata::Image {
								versions: Vec::new(),
							})),
						},
						total_size: 0,
						ty: FileType::CategoryCover,
						updated_at: chrono::Utc::now(),
						uploader_id: None,
					})
					.into_iter()
					.chain(item.artworks.iter().map(|artwork| UploadedFile {
						id: image_ids[&artwork.image_id],
						failed: None,
						name: format!("igdb/artwork/{}.jpg", artwork.image_id),
						owner_id: None,
						path: format!(
							"https://images.igdb.com/igdb/image/upload/t_1080p_2x/{}.jpg",
							artwork.image_id
						),
						status: UploadedFileStatus::Unqueued,
						metadata: UploadedFileMetadata {
							metadata: Some(uploaded_file_metadata::Metadata::Image(uploaded_file_metadata::Image {
								versions: Vec::new(),
							})),
						},
						total_size: 0,
						ty: FileType::CategoryArtwork,
						updated_at: chrono::Utc::now(),
						uploader_id: None,
					}))
			})
			.collect::<Vec<_>>();

		let uploaded_files_ids =
			utils::database::query("INSERT INTO uploaded_files (id, name, type, metadata, total_size, path, status) ")
				.push_values(&uploaded_files, |mut sep, item| {
					sep.push_bind(item.id);
					sep.push_bind(&item.name);
					sep.push_bind(item.ty);
					sep.push_bind(utils::database::Protobuf(item.metadata.clone()));
					sep.push_bind(item.total_size);
					sep.push_bind(&item.path);
					sep.push_bind(item.status);
				})
				.push("ON CONFLICT (id) DO NOTHING RETURNING id;")
				.build_query_single_scalar::<Ulid>()
				.fetch_all(&tx)
				.await
				.context("insert uploaded_files")?;

		let resp = resp
			.into_iter()
			.map(|item| InsertItem {
				id: Ulid::new(),
				igdb_id: item.id,
				name: item.name,
				alternative_names: item.alternative_names.into_iter().map(|x| x.name).collect::<Vec<_>>(),
				keywords: item
					.keywords
					.into_iter()
					.chain(item.genres)
					.chain(item.themes)
					.map(|x| x.name.to_lowercase())
					.collect::<Vec<_>>(),
				storyline: item.storyline,
				summary: item.summary,
				over_18: item.age_ratings.into_iter().any(|x| x.category == 2 && x.rating == 5), // PEGI 18
				cover_id: item.cover.map(|x| image_ids[&x.image_id]),
				rating: item.rating,
				updated_at: chrono::Utc::now(),
				artwork_ids: item.artworks.into_iter().map(|x| image_ids[&x.image_id]).collect::<Vec<_>>(),
				igdb_similar_game_ids: item.similar_games,
				websites: item.websites.into_iter().map(|x| x.url).collect::<Vec<_>>(),
			})
			.collect::<Vec<_>>();

		offset += resp.len();
		let count = resp.len();

		let categories = utils::database::query("INSERT INTO categories (id, igdb_id, name, aliases, keywords, storyline, summary, over_18, cover_id, rating, updated_at, artwork_ids, igdb_similar_game_ids, websites) ")
			.push_values(&resp, |mut sep, item| {
			sep.push_bind(item.id);
			sep.push_bind(item.igdb_id);
			sep.push_bind(&item.name);
			sep.push_bind(&item.alternative_names);
			sep.push_bind(&item.keywords);
			sep.push_bind(&item.storyline);
			sep.push_bind(&item.summary);
			sep.push_bind(item.over_18);
			sep.push_bind(item.cover_id);
			sep.push_bind(item.rating);
			sep.push_bind(item.updated_at);
			sep.push_bind(&item.artwork_ids);
			sep.push_bind(&item.igdb_similar_game_ids);
			sep.push_bind(&item.websites);
		})
			.push("ON CONFLICT (igdb_id) WHERE igdb_id IS NOT NULL DO UPDATE SET ")
			.push("name = EXCLUDED.name, ")
			.push("aliases = EXCLUDED.aliases, ")
			.push("keywords = EXCLUDED.keywords, ")
			.push("storyline = EXCLUDED.storyline, ")
			.push("summary = EXCLUDED.summary, ")
			.push("rating = EXCLUDED.rating, ")
			.push("updated_at = NOW(), ")
			.push("igdb_similar_game_ids = EXCLUDED.igdb_similar_game_ids, ")
			.push("websites = EXCLUDED.websites, ")
			.push("artwork_ids = EXCLUDED.artwork_ids RETURNING *;")
			.build_query_as::<Category>()
			.fetch_all(&tx)
			.await
			.context("insert categories")?;

		if categories.len() != count {
			tracing::warn!("igdb: categories count mismatch {} != {}", categories.len(), count);
		}

		let categories = categories
			.into_iter()
			.flat_map(|c| {
				c.cover_id
					.into_iter()
					.chain(c.artwork_ids.into_iter())
					.map(move |id| (id, c.id))
			})
			.collect::<HashMap<_, _>>();

		utils::database::query("WITH updated(id, category) AS (")
			.push_values(categories.iter().collect::<Vec<_>>(), |mut sep, item| {
				sep.push_bind(item.0).push_unseparated("::UUID");
				sep.push_bind(item.1).push_unseparated("::UUID");
			})
			.push(
				") UPDATE igdb_image SET category_id = updated.category FROM updated WHERE igdb_image.uploaded_file_id = updated.id;",
			)
			.build()
			.execute(&tx)
			.await
			.context("update igdb_image")?;

		tx.commit().await.context("commit")?;

		if config.process_images {
			let image_processor_config = global.config::<ImageUploaderConfig>();

			let tx = client
				.build_transaction()
				.isolation_level(IsolationLevel::ReadCommitted)
				.start()
				.await
				.context("start transaction image_jobs")?;

			let unqueued = utils::database::query(
				"UPDATE uploaded_files SET status = 'queued' WHERE id = ANY($1::UUID[]) AND status = 'unqueued' RETURNING id, path;",
			)
			.bind(uploaded_files_ids)
			.build_query_scalar::<(Ulid, String)>()
			.fetch_all(&tx)
			.await
			.context("update uploaded_files")?;

			if !unqueued.is_empty() {
				utils::database::query("INSERT INTO image_jobs (id, priority, task) ")
					.bind(image_processor_config.igdb_image_task_priority as i64)
					.push_values(unqueued, |mut sep, (id, path)| {
						sep.push_bind(id).push("$1").push_bind(utils::database::Protobuf(create_task(
							categories[&id],
							id,
							path,
							image_processor_config,
						)));
					})
					.push("ON CONFLICT (id) DO NOTHING;")
					.build()
					.execute(&tx)
					.await
					.context("insert image_jobs")?;

				tx.commit().await.context("commit image_jobs")?;
			}
		}

		tracing::debug!("igdb progress: {}/{}", offset, total);

		if count < 500 {
			tracing::info!("igdb: done");
			break;
		}

		timer.tick().await;
	}

	Ok(())
}

async fn get_access_token(config: &IgDbConfig) -> anyhow::Result<(String, i64)> {
	let client = reqwest::Client::new();
	let response = client
		.post("https://id.twitch.tv/oauth2/token")
		.form(&[
			("client_id", config.client_id.as_ref()),
			("client_secret", config.client_secret.as_ref()),
			("grant_type", "client_credentials"),
		])
		.send()
		.await
		.context("request")?;

	response.error_for_status_ref().context("response")?;

	#[derive(serde::Deserialize)]
	struct Response {
		access_token: String,
		expires_in: i64,
		token_type: String,
	}

	let response = response.json::<Response>().await.context("json")?;

	if response.token_type != "bearer" {
		anyhow::bail!("invalid token type: {}", response.token_type);
	}

	Ok((response.access_token, response.expires_in))
}

fn create_task(
	category_id: ulid::Ulid,
	id: ulid::Ulid,
	path: String,
	config: &ImageUploaderConfig,
) -> image_processor::Task {
	image_processor::Task {
		callback_subject: config.callback_subject.clone(),
		upscale: image_processor::task::Upscale::NoPreserveSource as i32,
		output_prefix: format!("categories/{category_id}/{id}"),
		scales: vec![720, 1080],
		input_image_scaling: true,
		limits: Some(image_processor::task::Limits {
			max_processing_time_ms: 60000,
			..Default::default()
		}),
		formats: vec![
			ImageFormat::AvifStatic as i32,
			ImageFormat::WebpStatic as i32,
			ImageFormat::PngStatic as i32,
		],
		input_path: path,
		resize_method: image_processor::task::ResizeMethod::Fit as i32,
		clamp_aspect_ratio: false,
		aspect_ratio: Some(image_processor::task::Ratio {
			numerator: 1,
			denominator: 1,
		}),
		resize_algorithm: image_processor::task::ResizeAlgorithm::Lanczos3 as i32,
	}
}
