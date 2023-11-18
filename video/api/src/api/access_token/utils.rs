use pb::ext::UlidExt;
use pb::scuffle::video::v1::AccessTokenGetRequest;
use tonic::Status;
use uuid::Uuid;
use video_common::database::AccessToken;

pub fn get_access_tokens<'a>(
	access_token: &'a AccessToken,
	req: &'a AccessTokenGetRequest,
) -> super::Result<sqlx::QueryBuilder<'a, sqlx::Postgres>> {
	let mut qb = sqlx::QueryBuilder::default();

	qb.push("SELECT * FROM access_tokens WHERE ");

	let mut seperated = qb.separated(" AND ");

	seperated.push("organization_id = ");
	seperated.push_bind_unseparated(Uuid::from(access_token.organization_id));

	if !req.ids.is_empty() {
		seperated.push("id IN ");
		seperated.push_bind_unseparated(req.ids.iter().map(|id| id.to_uuid()).collect::<Vec<_>>());
	}

	if let Some(options) = req.search_options.as_ref() {
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

		qb.push(" LIMIT ");
		qb.push_bind(limit);

		if options.reverse {
			qb.push(" ORDER BY id DESC");
		} else {
			qb.push(" ORDER BY id ASC");
		}
	} else {
		qb.push(" LIMIT 100 ORDER BY id ASC");
	}

	Ok(qb)
}

#[cfg(test)]
mod test {
	use pb::scuffle::video::v1::types::{SearchOptions, Tags};
	use ulid::Ulid;
	use video_common::database::AccessToken;

	use super::*;

	#[test]
	fn test_query_build() {
		let access_token = AccessToken {
			organization_id: common::database::Ulid(Ulid::new()),
			..Default::default()
		};

		let access_token_get_request = AccessTokenGetRequest {
			ids: vec![Ulid::new().into(), Ulid::new().into()],
			search_options: Some(SearchOptions {
				limit: 100,
				reverse: false,
				after_id: Some(Ulid::new().into()),
				tags: Some(pb::scuffle::video::v1::types::Tags {
					tags: vec![(String::from("test"), String::from("test"))].into_iter().collect(),
				}),
			}),
		};

		let query = super::get_access_tokens(&access_token, &access_token_get_request).unwrap();

		assert_eq!(
			query.sql(),
			"SELECT * FROM access_tokens WHERE organization_id = $1 AND id IN $2 AND id > $3 AND tags @> $4 LIMIT $5 ORDER BY id ASC"
		);
	}

	#[test]
	fn test_query_build_with_no_search_options() {
		let access_token = AccessToken {
			organization_id: common::database::Ulid(Ulid::new()),
			..Default::default()
		};

		let access_token_get_request = AccessTokenGetRequest {
			ids: vec![],
			search_options: None,
		};

		let query = get_access_tokens(&access_token, &access_token_get_request).unwrap();

		assert_eq!(
			query.sql(),
			"SELECT * FROM access_tokens WHERE organization_id = $1 LIMIT 100 ORDER BY id ASC"
		);
	}

	#[test]
	fn test_query_build_with_invalid_limit() {
		let access_token = AccessToken {
			organization_id: common::database::Ulid(Ulid::new()),
			..Default::default()
		};

		let access_token_get_request = AccessTokenGetRequest {
			ids: vec![],
			search_options: Some(SearchOptions {
				limit: 1001, // Invalid limit
				..Default::default()
			}),
		};

		let result = get_access_tokens(&access_token, &access_token_get_request);
		assert!(result.is_err());

		match result {
			Err(err) => assert_eq!(err.code(), tonic::Code::InvalidArgument),
			_ => unreachable!(),
		}
	}

	#[test]
	fn test_query_build_with_zero_limit() {
		let access_token = AccessToken {
			organization_id: common::database::Ulid(Ulid::new()),
			..Default::default()
		};

		let access_token_get_request = AccessTokenGetRequest {
			ids: vec![],
			search_options: Some(SearchOptions {
				limit: 0, // Zero limit should default to 100
				..Default::default()
			}),
		};

		let query = get_access_tokens(&access_token, &access_token_get_request).unwrap();

		assert_eq!(
			query.sql(),
			"SELECT * FROM access_tokens WHERE organization_id = $1 LIMIT $2 ORDER BY id ASC"
		);
	}

	#[test]
	fn test_query_build_with_reverse_order() {
		let access_token = AccessToken {
			organization_id: common::database::Ulid(Ulid::new()),
			..Default::default()
		};

		let access_token_get_request = AccessTokenGetRequest {
			ids: vec![],
			search_options: Some(SearchOptions {
				reverse: true, // Should order by id DESC
				..Default::default()
			}),
		};

		let query = get_access_tokens(&access_token, &access_token_get_request).unwrap();

		assert_eq!(
			query.sql(),
			"SELECT * FROM access_tokens WHERE organization_id = $1 LIMIT $2 ORDER BY id DESC"
		);
	}

	#[test]
	fn test_query_build_with_empty_tags() {
		let access_token = AccessToken {
			organization_id: common::database::Ulid(Ulid::new()),
			..Default::default()
		};

		let access_token_get_request = AccessTokenGetRequest {
			ids: vec![],
			search_options: Some(SearchOptions {
				tags: Some(Tags {
					tags: Default::default(),
				}), // Empty tags should not affect the query
				..Default::default()
			}),
		};

		let query = get_access_tokens(&access_token, &access_token_get_request).unwrap();

		assert_eq!(
			query.sql(),
			"SELECT * FROM access_tokens WHERE organization_id = $1 LIMIT $2 ORDER BY id ASC"
		);
	}

	#[test]
	fn test_query_build_with_ids_and_reverse_order() {
		let access_token = AccessToken {
			organization_id: common::database::Ulid(Ulid::new()),
			..Default::default()
		};

		let access_token_get_request = AccessTokenGetRequest {
			ids: vec![Ulid::new().into()],
			search_options: Some(SearchOptions {
				reverse: true,
				..Default::default()
			}),
		};

		let query = super::get_access_tokens(&access_token, &access_token_get_request).unwrap();

		assert_eq!(
			query.sql(),
			"SELECT * FROM access_tokens WHERE organization_id = $1 AND id IN $2 LIMIT $3 ORDER BY id DESC"
		);
	}

	#[test]
	fn test_query_build_with_tags_and_after_id() {
		let access_token = AccessToken {
			organization_id: common::database::Ulid(Ulid::new()),
			..Default::default()
		};

		let tags = Tags {
			tags: vec![(String::from("key"), String::from("value"))].into_iter().collect(),
		};

		let access_token_get_request = AccessTokenGetRequest {
			ids: vec![],
			search_options: Some(SearchOptions {
				after_id: Some(Ulid::new().into()),
				tags: Some(tags),
				..Default::default()
			}),
		};

		let query = super::get_access_tokens(&access_token, &access_token_get_request).unwrap();

		assert_eq!(
			query.sql(),
			"SELECT * FROM access_tokens WHERE organization_id = $1 AND id > $2 AND tags @> $3 LIMIT $4 ORDER BY id ASC"
		);
	}

	#[test]
	fn test_query_build_with_limit_and_ids() {
		let access_token = AccessToken {
			organization_id: common::database::Ulid(Ulid::new()),
			..Default::default()
		};

		let access_token_get_request = AccessTokenGetRequest {
			ids: vec![Ulid::new().into()],
			search_options: Some(SearchOptions {
				limit: 50,
				..Default::default()
			}),
		};

		let query = super::get_access_tokens(&access_token, &access_token_get_request).unwrap();

		assert_eq!(
			query.sql(),
			"SELECT * FROM access_tokens WHERE organization_id = $1 AND id IN $2 LIMIT $3 ORDER BY id ASC"
		);
	}

	#[test]
	fn test_query_build_with_all_combos() {
		let access_token = AccessToken {
			organization_id: common::database::Ulid(Ulid::new()),
			..Default::default()
		};

		let tags = Tags {
			tags: vec![(String::from("key"), String::from("value"))].into_iter().collect(),
		};

		let access_token_get_request = AccessTokenGetRequest {
			ids: vec![Ulid::new().into()],
			search_options: Some(SearchOptions {
				limit: 50,
				reverse: true,
				after_id: Some(Ulid::new().into()),
				tags: Some(tags),
			}),
		};

		let query = super::get_access_tokens(&access_token, &access_token_get_request).unwrap();

		assert_eq!(
			query.sql(),
			"SELECT * FROM access_tokens WHERE organization_id = $1 AND id IN $2 AND id > $3 AND tags @> $4 LIMIT $5 ORDER BY id DESC"
		);
	}
}
