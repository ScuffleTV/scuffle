CREATE TYPE image_type AS ENUM ('custom_thumbnail', 'profile_picture', 'offline_banner', 'role_badge');

CREATE TABLE users (
    id UUID NOT NULL PRIMARY KEY,
    username VARCHAR(32) NOT NULL,
    display_name VARCHAR(32) NOT NULL,
    display_color INT4 NOT NULL,
    password_hash VARCHAR(256) NOT NULL,
    totp_enabled BOOLEAN NOT NULL DEFAULT FALSE,
    totp_secret BYTES,
    two_fa_backup_codes INT4[],
    email VARCHAR(256) NOT NULL,
    email_verified BOOLEAN NOT NULL DEFAULT FALSE,
    last_login_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    profile_picture_id UUID,
    roles UUID[] NOT NULL DEFAULT '{}'::UUID[],

    -- Channel Settings
    channel_title VARCHAR(256),
    channel_live_viewer_count INT4,
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
    name VARCHAR(64) NOT NULL,
    revision INT4 NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE image_uploads (
    id UUID NOT NULL PRIMARY KEY,
    user_id UUID,
    ty image_type NOT NULL
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
ALTER TABLE users ADD CONSTRAINT users_profile_picture_id_fkey FOREIGN KEY (profile_picture_id) REFERENCES image_uploads (id) ON DELETE SET NULL;
ALTER TABLE users ADD CONSTRAINT users_channel_custom_thumbnail_id_fkey FOREIGN KEY (channel_custom_thumbnail_id) REFERENCES image_uploads (id) ON DELETE SET NULL;
ALTER TABLE users ADD CONSTRAINT users_channel_offline_banner_id_fkey FOREIGN KEY (channel_offline_banner_id) REFERENCES image_uploads (id) ON DELETE SET NULL;
ALTER TABLE users ADD CONSTRAINT users_channel_category_id_fkey FOREIGN KEY (channel_category_id) REFERENCES categories (id) ON DELETE SET NULL;

ALTER TABLE user_sessions ADD CONSTRAINT user_sessions_user_id_fkey FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE;

ALTER TABLE recordings ADD CONSTRAINT recordings_user_id_fkey FOREIGN KEY (channel_id) REFERENCES users (id) ON DELETE CASCADE;
ALTER TABLE recordings ADD CONSTRAINT recordings_custom_thumbnail_id_fkey FOREIGN KEY (custom_thumbnail_id) REFERENCES image_uploads (id) ON DELETE SET NULL;

ALTER TABLE channel_tags ADD CONSTRAINT channel_tags_user_id_fkey FOREIGN KEY (channel_id) REFERENCES users (id) ON DELETE CASCADE;

ALTER TABLE image_uploads ADD CONSTRAINT image_uploads_user_id_fkey FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE;

ALTER TABLE chat_messages ADD CONSTRAINT chat_messages_user_id_fkey FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE;
ALTER TABLE chat_messages ADD CONSTRAINT chat_messages_channel_id_fkey FOREIGN KEY (channel_id) REFERENCES users (id) ON DELETE CASCADE;

ALTER TABLE roles ADD CONSTRAINT roles_channel_id_fkey FOREIGN KEY (channel_id) REFERENCES users (id) ON DELETE CASCADE;
ALTER TABLE roles ADD CONSTRAINT roles_badge_id_fkey FOREIGN KEY (badge_id) REFERENCES image_uploads (id) ON DELETE SET NULL;

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

-- Image Upload Indexes

-- We want to be able to search for image uploads by user
CREATE INDEX image_uploads_user_id_idx ON image_uploads (user_id);

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