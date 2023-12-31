#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, sqlx::Type)]
pub enum FileType {
	#[sqlx(rename = "custom_thumbnail")]
	CustomThumbnail,
	#[sqlx(rename = "profile_picture")]
	ProfilePicture,
	#[sqlx(rename = "offline_banner")]
	OfflineBanner,
	#[sqlx(rename = "role_badge")]
	RoleBadge,
	#[sqlx(rename = "channel_role_badge")]
	ChannelRoleBadge,
}
