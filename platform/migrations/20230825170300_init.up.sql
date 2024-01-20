CREATE EXTENSION IF NOT EXISTS pg_trgm;

CREATE TYPE file_type AS ENUM ('profile_picture', 'category_cover', 'category_artwork');

CREATE TYPE uploaded_file_status AS ENUM ('unqueued', 'queued', 'failed', 'completed');

CREATE TABLE users (
    id UUID NOT NULL PRIMARY KEY,
    username VARCHAR(32) NOT NULL,
    password_hash VARCHAR(256) NOT NULL,

    -- Profile
    display_name VARCHAR(32) NOT NULL,
    display_color INT4 NOT NULL,
    profile_picture_id UUID,
    pending_profile_picture_id UUID,

    -- 2FA
    totp_enabled BOOLEAN NOT NULL DEFAULT FALSE,
    totp_secret BYTES,
    two_fa_backup_codes INT4[],

    -- Email
    email VARCHAR(256) NOT NULL,
    email_verified BOOLEAN NOT NULL DEFAULT FALSE,

    -- Time Info
    last_login_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    -- Permissions
    roles UUID[] NOT NULL DEFAULT '{}'::UUID[],

    -- Channel Settings
    channel_room_id UUID NOT NULL,
    channel_active_connection_id UUID DEFAULT NULL,
    channel_title VARCHAR(256),
    channel_live_viewer_count INT4 NOT NULL DEFAULT 0,
    channel_live_viewer_count_updated_at TIMESTAMPTZ,
    channel_description TEXT,
    channel_links JSONB NOT NULL DEFAULT '[]'::JSONB,
    channel_custom_thumbnail_id UUID,
    channel_offline_banner_id UUID,
    channel_category_id UUID,
    channel_stream_key VARCHAR(256),
    channel_role_order UUID[] NOT NULL DEFAULT '{}'::UUID[],
    channel_default_permissions BIGINT NOT NULL DEFAULT 0,
    channel_following_permission BIGINT NOT NULL DEFAULT 0,
    channel_last_live_at TIMESTAMPTZ
);

CREATE TABLE uploaded_files (
    id UUID NOT NULL PRIMARY KEY,
    owner_id UUID,
    uploader_id UUID,
    name VARCHAR(256) NOT NULL,
    type file_type NOT NULL,
    metadata BYTES NOT NULL,
    total_size INT8 NOT NULL,
    status uploaded_file_status NOT NULL,
    path VARCHAR(256) NOT NULL,
    failed TEXT,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE user_sessions (
    id UUID NOT NULL PRIMARY KEY,
    user_id UUID NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    last_used_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE two_fa_requests (
    id UUID NOT NULL PRIMARY KEY,
    user_id UUID NOT NULL,
    action BYTES NOT NULL
);

CREATE TABLE channel_tags (
    channel_id UUID NOT NULL,
    tag VARCHAR(64) NOT NULL,
    PRIMARY KEY (channel_id, tag)
);

CREATE TABLE recordings (
    id UUID NOT NULL PRIMARY KEY,
    channel_id UUID NOT NULL,
    title VARCHAR(256) NOT NULL,
    custom_thumbnail_id UUID,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    duration INT4,
    start_at TIMESTAMPTZ NOT NULL,
    view_count BIGINT NOT NULL DEFAULT 0,
    view_count_updated_at TIMESTAMPTZ,
    category_ids UUID[] NOT NULL DEFAULT '{}'::UUID[],
    category_times INTERVAL[] NOT NULL DEFAULT '{}'::INTERVAL[],
    end_at TIMESTAMPTZ
);

CREATE TABLE categories (
    id UUID NOT NULL PRIMARY KEY,
    igdb_id INT4,

    -- Searchable
    name TEXT NOT NULL,
    aliases TEXT[] NOT NULL DEFAULT '{}'::TEXT[],
    keywords TEXT[] NOT NULL DEFAULT '{}'::TEXT[],

    -- Non-searchable
    storyline TEXT,
    summary TEXT,
    over_18 BOOLEAN NOT NULL DEFAULT FALSE,
    cover_id UUID,
    rating FLOAT8 NOT NULL DEFAULT 0.0,
    artwork_ids UUID[] NOT NULL DEFAULT '{}'::UUID[],
    igdb_similar_game_ids INT4[] NOT NULL DEFAULT '{}'::INT4[],
    websites TEXT[] NOT NULL DEFAULT '{}'::TEXT[],

    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE igdb_image (
    image_id VARCHAR(64) NOT NULL PRIMARY KEY,
    uploaded_file_id UUID NOT NULL,
    category_id UUID
);

CREATE TABLE chat_messages (
    id UUID NOT NULL PRIMARY KEY,
    user_id UUID NOT NULL,
    channel_id UUID NOT NULL,
    content TEXT NOT NULL,
    deleted_at TIMESTAMPTZ
);

CREATE TABLE global_state (
    role_order UUID[] NOT NULL DEFAULT '{}'::UUID[],
    default_permissions BIGINT NOT NULL
);

CREATE TABLE roles (
    id UUID NOT NULL PRIMARY KEY,
    channel_id UUID,
    name VARCHAR(32) NOT NULL,
    description TEXT NOT NULL,
    allowed_permissions BIGINT NOT NULL DEFAULT 0 CHECK (allowed_permissions & denied_permissions = 0),
    denied_permissions BIGINT NOT NULL DEFAULT 0,
    badge_id UUID,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE channel_user (
    user_id UUID NOT NULL,
    channel_id UUID NOT NULL,
    roles UUID[] NOT NULL DEFAULT '{}'::UUID[],
    following BOOLEAN NOT NULL DEFAULT FALSE,
    notifications BOOLEAN NOT NULL DEFAULT FALSE,
    PRIMARY KEY (user_id, channel_id)
);

--- Relationships
ALTER TABLE users ADD CONSTRAINT users_profile_picture_id_fkey FOREIGN KEY (profile_picture_id) REFERENCES uploaded_files (id) ON DELETE SET NULL;
ALTER TABLE users ADD CONSTRAINT users_channel_custom_thumbnail_id_fkey FOREIGN KEY (channel_custom_thumbnail_id) REFERENCES uploaded_files (id) ON DELETE SET NULL;
ALTER TABLE users ADD CONSTRAINT users_channel_offline_banner_id_fkey FOREIGN KEY (channel_offline_banner_id) REFERENCES uploaded_files (id) ON DELETE SET NULL;
ALTER TABLE users ADD CONSTRAINT users_channel_category_id_fkey FOREIGN KEY (channel_category_id) REFERENCES categories (id) ON DELETE SET NULL;

ALTER TABLE user_sessions ADD CONSTRAINT user_sessions_user_id_fkey FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE;

ALTER TABLE recordings ADD CONSTRAINT recordings_user_id_fkey FOREIGN KEY (channel_id) REFERENCES users (id) ON DELETE CASCADE;
ALTER TABLE recordings ADD CONSTRAINT recordings_custom_thumbnail_id_fkey FOREIGN KEY (custom_thumbnail_id) REFERENCES uploaded_files (id) ON DELETE SET NULL;

ALTER TABLE channel_tags ADD CONSTRAINT channel_tags_user_id_fkey FOREIGN KEY (channel_id) REFERENCES users (id) ON DELETE CASCADE;

ALTER TABLE uploaded_files ADD CONSTRAINT uploaded_files_user_id_fkey FOREIGN KEY (owner_id) REFERENCES users (id);
ALTER TABLE uploaded_files ADD CONSTRAINT uploaded_files_uploader_id_fkey FOREIGN KEY (uploader_id) REFERENCES users (id);

ALTER TABLE chat_messages ADD CONSTRAINT chat_messages_user_id_fkey FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE;
ALTER TABLE chat_messages ADD CONSTRAINT chat_messages_channel_id_fkey FOREIGN KEY (channel_id) REFERENCES users (id) ON DELETE CASCADE;

ALTER TABLE roles ADD CONSTRAINT roles_channel_id_fkey FOREIGN KEY (channel_id) REFERENCES users (id) ON DELETE CASCADE;
ALTER TABLE roles ADD CONSTRAINT roles_badge_id_fkey FOREIGN KEY (badge_id) REFERENCES uploaded_files (id) ON DELETE SET NULL;

ALTER TABLE channel_user ADD CONSTRAINT channel_user_user_id_fkey FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE;
ALTER TABLE channel_user ADD CONSTRAINT channel_user_channel_id_fkey FOREIGN KEY (channel_id) REFERENCES users (id) ON DELETE CASCADE;

--- Indexes

-- User Indexes

-- We want to be able to search for users by username (fuzzy search)
CREATE INVERTED INDEX users_username_idx ON users (username gin_trgm_ops);

-- We want to find all the users that have a certain role
CREATE INVERTED INDEX users_roles_idx ON users (roles);

-- Channel Indexes
-- We want to be able to do full text search on titles and descriptions
CREATE INDEX users_channel_title_description_idx ON users USING GIN (to_tsvector('english', channel_title || channel_description));
CREATE INDEX users_channel_live_viewer_count ON users (channel_live_viewer_count DESC) WHERE channel_live_viewer_count IS NOT NULL;
CREATE INDEX users_channel_last_live_at_idx ON users (channel_last_live_at) WHERE channel_last_live_at IS NOT NULL;
CREATE INDEX users_channel_category_id_idx ON users (channel_category_id) WHERE channel_category_id IS NOT NULL;
CREATE INDEX users_channel_room_id_idx ON users (channel_room_id);

-- User Session Indexes

-- We want to be able to find all sessions for a user
CREATE INDEX user_sessions_user_id_idx ON user_sessions (user_id);

-- Channel Tag Indexes

-- We want to be able to find all tags for a channel
CREATE INDEX channel_tags_channel_id_idx ON channel_tags (channel_id);
-- We want to be able to find all channels with a certain tag
CREATE INDEX channel_tags_tag_idx ON channel_tags (tag);

-- Recording Indexes

-- We want to be able to search for recordings by title (full text search)
CREATE INDEX recordings_title_idx ON recordings USING GIN (to_tsvector('english', title));

-- We want to be able to search for recordings by category
CREATE INVERTED INDEX recordings_category_ids_idx ON recordings (category_ids);
-- We want to be able to search for recordings by user
CREATE INDEX recordings_channel_id_idx ON recordings (channel_id);
-- We want to be able to search for recordings by view count
CREATE INDEX recordings_view_count_idx ON recordings (view_count DESC);
-- We want to be able to search for recordings by start time
CREATE INDEX recordings_start_at_idx ON recordings (start_at);

-- Category Indexes

-- We want to be able to search for categories by name (fuzzy search)
CREATE INVERTED INDEX categories_name_idx ON categories (name gin_trgm_ops);
CREATE INVERTED INDEX categories_aliases_idx ON categories (aliases);
CREATE INVERTED INDEX categories_keywords_idx ON categories (keywords);

-- We want to be able to search for categories by IGDB ID
CREATE UNIQUE INDEX categories_igdb_id_idx ON categories (igdb_id) WHERE igdb_id IS NOT NULL;

-- We want to be able to search for categories by rating
CREATE INDEX categories_rating_idx ON categories (rating DESC);

-- Image Upload Indexes

-- We want to be able to search for image uploads by user
CREATE INDEX uploaded_files_user_id_idx ON uploaded_files (owner_id, type);

-- Chat Message Indexes

-- We want to be able to search for chat messages by user in a channel
CREATE INDEX chat_messages_channel_id_user_id_idx ON chat_messages (channel_id, user_id);

-- Role Indexes

-- We want to be able to search for roles by channel
CREATE INDEX roles_channel_id_idx ON roles (channel_id);

-- Channel User Indexes

-- We want to be able to find all users that have notifications enabled for a channel
CREATE INDEX channel_user_channel_id_following_notifcations_idx ON channel_user (channel_id, following, notifications);
CREATE INDEX channel_user_channel_id_roles_idx ON channel_user (channel_id, roles);

-- Default global state
INSERT INTO global_state (default_permissions) VALUES (0);

-- Image Processor

CREATE TABLE image_jobs (
    id UUID NOT NULL PRIMARY KEY,
    priority INTEGER NOT NULL DEFAULT 1,
    claimed_by UUID,
    hold_until TIMESTAMPTZ,
    task BYTES NOT NULL
);

CREATE INDEX image_jobs_idx_hold_until ON image_jobs (hold_until);

CREATE INDEX image_jobs_idx_priority_id ON image_jobs (priority DESC, id DESC);
