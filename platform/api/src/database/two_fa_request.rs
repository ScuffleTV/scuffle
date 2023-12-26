use std::sync::Arc;

use chrono::{Duration, Utc};
use common::database::{Protobuf, Ulid};
use pb::ext::UlidExt;
use pb::scuffle::platform::internal::two_fa::two_fa_request_action::{ChangePassword, Login};
use pb::scuffle::platform::internal::two_fa::TwoFaRequestAction;

use super::{Session, User};
use crate::global::ApiGlobal;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct TwoFaRequest {
	pub id: Ulid,
	pub user_id: Ulid,
	pub action: Protobuf<TwoFaRequestAction>,
}

#[allow(async_fn_in_trait)]
pub trait TwoFaRequestActionTrait<G: ApiGlobal> {
	type Result;

	async fn execute(self, global: &Arc<G>, user_id: Ulid) -> Self::Result;
}

impl<G: ApiGlobal> TwoFaRequestActionTrait<G> for Login {
	type Result = sqlx::Result<Session>;

	async fn execute(self, global: &Arc<G>, user_id: Ulid) -> Self::Result {
		let expires_at = Utc::now() + Duration::seconds(self.login_duration as i64);

		// TODO: maybe look to batch this
		let mut tx = global.db().begin().await?;

		let session: Session = sqlx::query_as(
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
		.bind(Ulid::from(ulid::Ulid::new()))
		.bind(user_id)
		.bind(expires_at)
		.fetch_one(tx.as_mut())
		.await?;

		sqlx::query(
			r#"
			UPDATE users
			SET
				last_login_at = NOW()
			WHERE id = $1
			"#,
		)
		.bind(user_id)
		.execute(tx.as_mut())
		.await?;

		tx.commit().await?;

		Ok(session)
	}
}

impl<G: ApiGlobal> TwoFaRequestActionTrait<G> for ChangePassword {
	type Result = sqlx::Result<()>;

	async fn execute(self, global: &Arc<G>, user_id: Ulid) -> sqlx::Result<()> {
		let mut tx = global.db().begin().await?;

		let user: User = sqlx::query_as(
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
		.fetch_one(tx.as_mut())
		.await?;

		// Delete all sessions except current
		sqlx::query(
			r#"
			DELETE FROM
				user_sessions
			WHERE
				user_id = $1
				AND id != $2
			"#,
		)
		.bind(user.id)
		.bind(Ulid::from(self.current_session_id.into_ulid()))
		.execute(tx.as_mut())
		.await?;

		tx.commit().await?;

		Ok(())
	}
}
