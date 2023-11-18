#[derive(Debug, Default, sqlx::Type, Clone, Copy, PartialEq)]
#[sqlx(type_name = "room_status")]
pub enum RoomStatus {
	#[sqlx(rename = "OFFLINE")]
	#[default]
	Offline,
	#[sqlx(rename = "WAITING_FOR_TRANSCODER")]
	WaitingForTranscoder,
	#[sqlx(rename = "READY")]
	Ready,
}

impl From<RoomStatus> for i32 {
	fn from(value: RoomStatus) -> Self {
		pb::scuffle::video::v1::types::RoomStatus::from(value) as i32
	}
}

impl From<RoomStatus> for pb::scuffle::video::v1::types::RoomStatus {
	fn from(value: RoomStatus) -> Self {
		match value {
			RoomStatus::Offline => pb::scuffle::video::v1::types::RoomStatus::Offline,
			RoomStatus::WaitingForTranscoder => pb::scuffle::video::v1::types::RoomStatus::WaitingForTranscoder,
			RoomStatus::Ready => pb::scuffle::video::v1::types::RoomStatus::Ready,
		}
	}
}
