use pb::ext::UlidExt;
use pb::scuffle::video::v1::types::SearchOptions;
use tonic::Status;
use uuid::Uuid;

pub fn organization_id(
	seperated: &mut sqlx::query_builder::Separated<'_, '_, sqlx::Postgres, &str>,
	organization_id: common::database::Ulid,
) {
	seperated.push("organization_id = ");
	seperated.push_bind_unseparated(Uuid::from(organization_id));
}

pub fn ids(seperated: &mut sqlx::query_builder::Separated<'_, '_, sqlx::Postgres, &str>, ids: &[pb::scuffle::types::Ulid]) {
	if !ids.is_empty() {
		seperated.push("id = ANY(");
		seperated.push_bind_unseparated(ids.iter().map(pb::ext::UlidExt::to_uuid).collect::<Vec<_>>());
		seperated.push_unseparated(")");
	}
}

pub fn search_options(
	seperated: &mut sqlx::query_builder::Separated<'_, '_, sqlx::Postgres, &str>,
	search_options: Option<&SearchOptions>,
) -> tonic::Result<()> {
	if let Some(options) = search_options {
		if let Some(after_id) = options.after_id.as_ref() {
			seperated.push("id > ");
			seperated.push_bind_unseparated(after_id.to_uuid());
		}

		if let Some(tags) = options.tags.as_ref() {
			if !tags.tags.is_empty() {
				seperated.push("tags @> ");
				seperated.push_bind_unseparated(
					serde_json::to_value(&tags.tags).map_err(|_| Status::internal("failed to serialize tags"))?,
				);
			}
		}

		let limit = if options.limit == 0 {
			100
		} else if options.limit >= 1 && options.limit <= 1000 {
			options.limit
		} else {
			return Err(Status::invalid_argument("limit must be between 1 and 1000"));
		};

		seperated.push_unseparated(" LIMIT ");
		seperated.push_bind_unseparated(limit);

		if options.reverse {
			seperated.push_unseparated(" ORDER BY id DESC");
		} else {
			seperated.push_unseparated(" ORDER BY id ASC");
		}
	} else {
		seperated.push_unseparated(" LIMIT 100 ORDER BY id ASC");
	}

	Ok(())
}