use postgres_types::{FromSql, ToSql};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, ToSql, FromSql)]
#[postgres(name = "file_type")]
pub enum FileType {
	#[postgres(name = "profile_picture")]
	ProfilePicture,
	#[postgres(name = "category_cover")]
	CategoryCover,
	#[postgres(name = "category_artwork")]
	CategoryArtwork,
}
