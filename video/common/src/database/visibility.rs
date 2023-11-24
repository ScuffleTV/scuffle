#[derive(Debug, Default, sqlx::Type, Clone, Copy, PartialEq)]
#[sqlx(type_name = "visibility")]
pub enum Visibility {
	#[sqlx(rename = "PUBLIC")]
	#[default]
	Public,
	#[sqlx(rename = "PRIVATE")]
	Private,
}

impl From<Visibility> for i32 {
	fn from(value: Visibility) -> Self {
		pb::scuffle::video::v1::types::Visibility::from(value) as i32
	}
}

impl From<Visibility> for pb::scuffle::video::v1::types::Visibility {
	fn from(value: Visibility) -> Self {
		match value {
			Visibility::Public => pb::scuffle::video::v1::types::Visibility::Public,
			Visibility::Private => pb::scuffle::video::v1::types::Visibility::Private,
		}
	}
}

impl From<pb::scuffle::video::v1::types::Visibility> for Visibility {
	fn from(value: pb::scuffle::video::v1::types::Visibility) -> Self {
		match value {
			pb::scuffle::video::v1::types::Visibility::Public => Visibility::Public,
			pb::scuffle::video::v1::types::Visibility::Private => Visibility::Private,
		}
	}
}
