use bitmask_enum::bitmask;
use ulid::Ulid;

#[derive(Debug, Clone, Default, postgres_from_row::FromRow)]
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
	/// Upload Profile Pictures
	UploadProfilePicture,
}

impl<'a> postgres_types::FromSql<'a> for RolePermission {
	fn accepts(ty: &postgres_types::Type) -> bool {
		<i64 as postgres_types::FromSql>::accepts(ty)
	}

	fn from_sql(ty: &postgres_types::Type, raw: &'a [u8]) -> Result<Self, Box<dyn std::error::Error + Sync + Send>> {
		<i64 as postgres_types::FromSql>::from_sql(ty, raw).map(Self::from)
	}
}

impl postgres_types::ToSql for RolePermission {
	postgres_types::to_sql_checked!();

	fn to_sql(
		&self,
		ty: &postgres_types::Type,
		out: &mut bytes::BytesMut,
	) -> Result<postgres_types::IsNull, Box<dyn std::error::Error + Sync + Send>> {
		<i64 as postgres_types::ToSql>::to_sql(&self.bits(), ty, out)
	}

	fn accepts(ty: &postgres_types::Type) -> bool {
		<i64 as postgres_types::ToSql>::accepts(ty)
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
	pub fn has_permission(self, other: Self) -> bool {
		(self & Self::Admin == Self::Admin) || (self & other == other)
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
	pub const fn merge(self, other: Self) -> Self {
		self.or(other)
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
	pub const fn remove(self, other: Self) -> Self {
		self.and(other.not())
	}

	pub const fn merge_with_role(self, role: &Role) -> Self {
		self.merge(role.allowed_permissions).remove(role.denied_permissions)
	}
}
