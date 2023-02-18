use bitmask_enum::bitmask;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct Model {
    pub id: i64,                   // bigint, primary key
    pub owner_id: i64,             // bigint, foreign key -> users.id
    pub channel_id: Option<i64>,   // bigint?, foreign key -> channels.id
    pub name: String,              // varchar(32)
    pub description: String,       // text
    pub rank: i32,                 // int, 0 is lowest rank, 255 is highest rank
    pub allowed_permissions: i64,  // bigint, bitmask of permissions
    pub denied_permissions: i64,   // bigint, bitmask of permissions
    pub created_at: DateTime<Utc>, // timestamptz
}

#[bitmask(i64)]
pub enum Permission {
    View,             // Can view this channel
    Watch,            // Can watch videos on this channel
    Read,             // Can read chat rooms on this channel
    Talk,             // Can talk in chat rooms on this channel
    EditChannel,      // Can edit this channel channel
    DeleteChannel,    // Can delete this channel channel
    GoLive,           // Can go live on this channel channel
    ChatRoomBypass, // Can bypass chat room restrictions on their own channel (follow only, sub only, cannot be banned from chat rooms, ect)
    ChatRoomModerate, // Can moderate chat rooms on this channel
    ChatRoomManage, // Can create/edit/delete chat rooms on this channel
    ManageUsers,    // Can ban/unban users on this channel
    GrantRoles,     // Can grant roles to users on this channel channel
    ManageRoles,    // Can create/edit/delete roles on this channel
}
