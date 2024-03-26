use std::sync::Arc;

use chrono::{Duration, Utc};
use pb::ext::UlidExt;
use pb::scuffle::platform::internal::two_fa::two_fa_request_action::{ChangePassword, Login};
use pb::scuffle::platform::internal::two_fa::TwoFaRequestAction;
use ulid::Ulid;
use utils::database::protobuf;

use super::{Session, User};
use crate::global::ApiGlobal;

#[derive(Debug, Clone, postgres_from_row::FromRow)]
pub struct TwoFaRequest {
	pub id: Ulid,
	pub user_id: Ulid,

	#[from_row(from_fn = "protobuf")]
	pub action: TwoFaRequestAction,
}

#[allow(async_fn_in_trait)]
pub trait TwoFaRequestActionTrait<G: ApiGlobal> {
	type Result;

	async fn execute(self, global: &Arc<G>, user_id: Ulid) -> Self::Result;
}

impl<G: ApiGlobal> TwoFaRequestActionTrait<G> for Login {
	type Result = Result<Session, utils::database::deadpool_postgres::PoolError>;

	async fn execute(self, global: &Arc<G>, user_id: Ulid) -> Self::Result {
		let expires_at = Utc::now() + Duration::seconds(self.login_duration as i64);

		// TODO: maybe look to batch this
		let mut client = global.db().get().await?;
		let tx = client.transaction().await?;

		let session = utils::database::query(
			r#"
			INSERT INTO user_sessions (
				id,
				user_id,
				expires_at
			) VALUES (
				$1,
				$2,
				$3
			) RETURNING *
			"#,
		)
		.bind(ulid::Ulid::new())
		.bind(user_id)
		.bind(expires_at)
		.build_query_as()
		.fetch_one(&tx)
		.await?;

		utils::database::query(
			r#"
			UPDATE users
			SET
				last_login_at = NOW()
			WHERE id = $1
			"#,
		)
		.bind(user_id)
		.build()
		.execute(&tx)
		.await?;

		tx.commit().await?;

		Ok(session)
	}
}

impl<G: ApiGlobal> TwoFaRequestActionTrait<G> for ChangePassword {
	type Result = Result<(), utils::database::deadpool_postgres::PoolError>;

	async fn execute(self, global: &Arc<G>, user_id: Ulid) -> Self::Result {
		let mut client = global.db().get().await?;
		let tx = client.transaction().await?;

		let user: User = utils::database::query(
			r#"
			UPDATE
				users
			SET
				password_hash = $1
			WHERE
				id = $2
			RETURNING *
			"#,
		)
		.bind(self.new_password_hash)
		.bind(user_id)
		.build_query_as()
		.fetch_one(&tx)
		.await?;

		// Delete all sessions except current
		utils::database::query(
			r#"
			DELETE FROM
				user_sessions
			WHERE
				user_id = $1
				AND id != $2
			"#,
		)
		.bind(user.id)
		.bind(self.current_session_id.into_ulid())
		.build()
		.execute(&tx)
		.await?;

		tx.commit().await?;

		Ok(())
	}
}
