use bitmask_enum::bitmask;
use common::database::Ulid;

#[derive(Debug, Clone, Default, sqlx::FromRow)]
/// A role that can be granted to a user.
/// Roles can allow or deny permissions to a user.
pub struct Role {
	/// The unique identifier for the role.
	pub id: Ulid,
	/// The channel this role is for. None for global roles.
	pub channel_id: Option<Ulid>,
	/// The name of the role.
	pub name: String,
	/// The description of the role.
	pub description: String,
	/// The permissions granted by this role.
	pub allowed_permissions: RolePermission,
	/// The permissions denied by this role.
	pub denied_permissions: RolePermission,
}

#[bitmask(i64)]
pub enum RolePermission {
	/// Can do anything
	Admin,
	/// Can start streaming
	GoLive,
	/// Has access to transcoding
	StreamTranscoding,
	/// Has access to recording
	StreamRecording,
}

impl sqlx::Decode<'_, sqlx::Postgres> for RolePermission {
	fn decode(value: sqlx::postgres::PgValueRef<'_>) -> Result<Self, Box<dyn std::error::Error + 'static + Send + Sync>> {
		<i64 as sqlx::Decode<sqlx::Postgres>>::decode(value).map(Self::from)
	}
}

impl sqlx::Type<sqlx::Postgres> for RolePermission {
	fn type_info() -> sqlx::postgres::PgTypeInfo {
		<i64 as sqlx::Type<sqlx::Postgres>>::type_info()
	}
}

impl Default for RolePermission {
	fn default() -> Self {
		Self::none()
	}
}

impl RolePermission {
	/// Checks if the current permission set has the given permission.
	/// Admin permissions always return true. Otherwise, the permission is
	/// checked against the current permission set.
	pub fn has_permission(&self, other: Self) -> bool {
		(*self & Self::Admin == Self::Admin) || (*self & other == other)
	}

	/// Merge the given permissions.
	///
	/// # Example
	///
	/// self: `10011`
	/// other: `11000`
	/// result: `11011`
	///
	/// ## Calculation
	///
	/// `10011 | 11000 = 11011`
	pub fn merge(&self, other: &Self) -> Self {
		*self | *other
	}

	/// Remove the given permissions from the current.
	///
	/// # Example
	///
	/// self: `10011`
	/// other: `10001`
	/// result: `00010`
	///
	/// ## Calculation
	///
	/// `10011 & !10001 = 10011 & 01110 = 00010`
	pub fn remove(&self, other: &Self) -> Self {
		*self & !*other
	}

	pub fn merge_with_role(&self, role: &Role) -> Self {
		self.merge(&role.allowed_permissions).remove(&role.denied_permissions)
	}
}
