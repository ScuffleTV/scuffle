use postgres_types::{FromSql, ToSql};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, ToSql, FromSql)]
#[postgres(name = "file_type")]
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
	#[postgres(name = "category_cover")]
	CategoryCover,
	#[postgres(name = "category_artwork")]
	CategoryArtwork,
}
