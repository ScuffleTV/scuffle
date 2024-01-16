use postgres_types::{FromSql, ToSql};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, ToSql, FromSql)]
pub enum FileType {
	#[postgres(name = "custom_thumbnail")]
	CustomThumbnail,
	#[postgres(name = "profile_picture")]
	ProfilePicture,
	#[postgres(name = "offline_banner")]
	OfflineBanner,
	#[postgres(name = "role_badge")]
	RoleBadge,
	#[postgres(name = "channel_role_badge")]
	ChannelRoleBadge,
}
