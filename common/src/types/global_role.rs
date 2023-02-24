use bitmask_enum::bitmask;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Default)]
pub struct Model {
    pub id: i64,                   // bigint, primary key
    pub name: String,              // varchar(32)
    pub description: String,       // text
    pub rank: i32,                 // int
    pub allowed_permissions: i64,  // bigint, bitfield -> Permission
    pub denied_permissions: i64,   // bigint, bitfield -> Permission
    pub created_at: DateTime<Utc>, // timestamptz
}

#[bitmask(i64)]
pub enum Permission {
    UseChannels, // Can create their own channels, edit their own channels, delete their own channels
    GoLive,      // Can go live on their own channels
    ChatRoomBypass, // Can bypass chat room restrictions globally (follow only, sub only, cannot be banned from chat rooms, ect)
    ManageUsers,    // Can ban/unban users.
    ManageChannels, // Can edit/delete any channel, can create channels for other users
    GrantRoles,     // Can grant roles to users
    ManageRoles,    // Can create/edit/delete roles
}
