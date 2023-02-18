# Database Design

## Overview

### User Structure

- `id` - bigint - Unique identifier for the user
- `username` - varchar(32) - Username of the user (unique)
- `password_hash` - varchar(255) - Hash of the user's password (argon2)
- `email` - varchar(255) - Email of the user
- `email_verified` - boolean - Whether the user's email has been verified
- `created_at` - timestamptz - When the user was created

### Session Structure

- `id` - bigint - Unique identifier for the session
- `user_id` - bigint - User ID of the user who owns the session
- `token` - char(64) - Token of the session
- `created_at` - timestamptz - When the session was created
- `expires_at` - timestamptz - When the session expires
- `last_used_at` - timestamptz - When the session was last used

### Global Role Structure

- `id` - bigint - Unique identifier for the global role
- `name` - varchar(32) - Name of the global role (unique)
- `description` - text - Description of the global role
- `rank` - integer - Rank of the global role (higher = priority)
- `allowed_permissions` - bigint - Bitmask of allowed permissions
- `denied_permissions` - bigint - Bitmask of denied permissions
- `created_at` - timestamptz - When the global role was created

### Global Role Grant Structure

- `id` - bigint - Unique identifier for the global role grant
- `user_id` - bigint - User ID of the user who owns the global role grant
- `global_role_id` - bigint - Global role ID of the global role granted
- `created_at` - timestamptz - When the global role grant was created

### Global Ban Structure

- `id` - bigint - Unique identifier for the global ban
- `user_id` - bigint - User ID of the user who owns the global ban
- `mode` - bigint - Bitmask of the ban mode
- `reason` - text - Reason for the global ban
- `expires_at` - timestamptz? - When the global ban expires
- `created_at` - timestamptz - When the global ban was created

### Chat Room Structure

- `id` - bigint - Unique identifier for the chat room
- `owner_id` - bigint - User ID of the user who owns the chat room
- `name` - varchar(32) - Name of the chat room (unique per owner)
- `description` - text - Description of the chat room
- `created_at` - timestamptz - When the chat room was created
- `deleted_at` - timestamptz? - When the chat room was deleted

### Channel Structure

- `id` - bigint - Unique identifier for the channel
- `owner_id` - bigint - User ID of the user who owns the channel
- `name` - varchar(32) - Name of the channel (unique per owner)
- `description` - text - Description of the channel
- `stream_key` - char(25) - Stream key of the channel
- `chat_room_id` - bigint - Chat room ID of the chat room associated with the channel
- `last_live_at` - timestamptz? - When the channel was last live
- `created_at` - timestamptz - When the channel was created
- `deleted_at` - timestamptz? - When the channel was deleted

### Channel Role Structure

- `id` - bigint - Unique identifier for the channel role
- `owner_id` - bigint - User ID of the user who owns the channel role
- `channel_id` - bigint - Channel ID of the channel the channel role is for
- `name` - varchar(32) - Name of the channel role (unique per owner and channel)
- `description` - text - Description of the channel role
- `rank` - integer - Rank of the channel role (higher = priority)
- `allowed_permissions` - bigint - Bitmask of allowed permissions
- `denied_permissions` - bigint - Bitmask of denied permissions
- `created_at` - timestamptz - When the channel role was created

### Channel Role Grant Structure

- `id` - bigint - Unique identifier for the channel role grant
- `user_id` - bigint - User ID of the user who owns the channel role grant
- `channel_role_id` - bigint - Channel role ID of the channel role granted
- `created_at` - timestamptz - When the channel role grant was created

### Stream Structure

- `id` - bigint - Unique identifier for the stream
- `channel_id` - bigint - Channel ID of the channel the stream is for
- `title` - varchar(255) - Title of the stream
- `description` - text - Description of the stream
- `created_at` - timestamptz - When the stream was created
- `started_at` - timestamptz? - When the stream started
- `ended_at` - timestamptz? - When the stream ended

### Follow Structure

- `id` - bigint - Unique identifier for the follow
- `follower_id` - bigint - User ID of the user who is following
- `followed_id` - bigint - User ID of the user who is being followed
- `channel_id` - bigint? - Channel ID of the channel being followed
- `created_at` - timestamptz - When the follow was created

### Channel Ban Structure

- `id` - bigint - Unique identifier for the channel ban
- `owner_id` - bigint - User ID of the user who owns the channel
- `target_id` - bigint - User ID of the user who is being banned
- `channel_id` - bigint? - Channel ID of the channel the channel ban is for
- `mode` - bigint - Bitmask of the ban mode
- `reason` - text - Reason for the channel ban
- `expires_at` - timestamptz? - When the channel ban expires
- `created_at` - timestamptz - When the channel ban was created

### Chat Message Structure

- `id` - bigint - Unique identifier for the chat message
- `chat_room_id` - bigint - Chat room ID of the chat room the chat message is for
- `author_id` - bigint - User ID of the user who authored the chat message
- `message` - text - Message of the chat message
- `created_at` - timestamptz - When the chat message was created
