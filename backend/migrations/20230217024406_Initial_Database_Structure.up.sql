CREATE TABLE users (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    username varchar(32) NOT NULL,
    display_name varchar(32) NOT NULL,
    password_hash varchar(255) NOT NULL,
    email varchar(255) NOT NULL,
    email_verified boolean NOT NULL DEFAULT FALSE,

    -- Stream state
    stream_key varchar(255) NOT NULL,
    stream_title varchar(255) NOT NULL DEFAULT '',
    stream_description text NOT NULL DEFAULT '',
    stream_transcoding_enabled boolean NOT NULL DEFAULT FALSE,
    stream_recording_enabled boolean NOT NULL DEFAULT FALSE,

    -- Timestamps
    created_at timestamptz NOT NULL DEFAULT NOW(),
    last_login_at timestamptz NOT NULL DEFAULT NOW()
);

CREATE TABLE sessions (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id uuid NOT NULL, -- foreign key to users(id)
    invalidated_at timestamptz DEFAULT NULL,
    -- Timestamps
    created_at timestamptz NOT NULL DEFAULT NOW(),
    expires_at timestamptz NOT NULL DEFAULT NOW(),
    last_used_at timestamptz NOT NULL DEFAULT NOW()
);

CREATE TABLE global_roles (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    name varchar(32) NOT NULL,
    description text NOT NULL,
    rank int NOT NULL CHECK (rank >= -1),
    -- allowed_permissions & denied_permissions = 0
    -- We only need to check one of them
    allowed_permissions bigint NOT NULL DEFAULT 0 CHECK (allowed_permissions & denied_permissions = 0),
    denied_permissions bigint NOT NULL DEFAULT 0,
    -- Timestamps
    created_at timestamptz NOT NULL DEFAULT NOW()
);

CREATE TABLE global_role_grants (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id uuid NOT NULL, -- foreign key to users(id)
    global_role_id uuid NOT NULL, -- foreign key to global_roles(id)
    -- Timestamps
    created_at timestamptz NOT NULL DEFAULT NOW()
);

CREATE TABLE channel_roles (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    channel_id uuid NOT NULL, -- foreign key to users(id)
    name varchar(32) NOT NULL,
    description text NOT NULL,
    rank int NOT NULL CHECK (rank >= -1),
    -- allowed_permissions & denied_permissions = 0
    -- We only need to check one of them
    allowed_permissions bigint NOT NULL DEFAULT 0 CHECK (allowed_permissions & denied_permissions = 0),
    denied_permissions bigint NOT NULL DEFAULT 0,
    -- Timestamps
    created_at timestamptz NOT NULL DEFAULT NOW()
);

CREATE TABLE channel_role_grants (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id uuid NOT NULL, -- foreign key to users(id)
    channel_role_id uuid NOT NULL, -- foreign key to channel_roles(id)
    -- Timestamps
    created_at timestamptz NOT NULL DEFAULT NOW()
);

CREATE TABLE streams (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    channel_id uuid NOT NULL, -- foreign key to users(id)
    title varchar(255) NOT NULL,
    description text NOT NULL,
    recorded boolean NOT NULL DEFAULT FALSE,
    transcoded boolean NOT NULL DEFAULT FALSE,
    deleted boolean NOT NULL DEFAULT FALSE,
    state int NOT NULL DEFAULT 0, -- 0 = not ready, 1 = ready, 2 = stopped, 3 = stopped resumable, 4 = failed, 5 = was ready
    ingest_address varchar(255) NOT NULL,
    connection_id uuid NOT NULL,
    -- Timestamps
    created_at timestamptz NOT NULL DEFAULT NOW(),
    updated_at timestamptz DEFAULT NULL, -- NULL = not started (last bitrate is report)
    ended_at timestamptz NOT NULL DEFAULT NOW() + interval '5 minutes'
);

CREATE TABLE stream_bitrate_updates (
    stream_id uuid NOT NULL, -- foreign key to streams(id)
    video_bitrate bigint NOT NULL,
    audio_bitrate bigint NOT NULL,
    metadata_bitrate bigint NOT NULL,
    -- Timestamps
    created_at timestamptz NOT NULL DEFAULT NOW()
);

CREATE TABLE stream_variants (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    stream_id uuid NOT NULL, -- foreign key to streams(id)
    name varchar(255) NOT NULL,
    video_framerate int, -- null = audio only
    video_width int, -- null = audio only
    video_height int, -- null = audio only
    video_bitrate int, -- null = audio only
    video_codec varchar(255), -- null = audio only
    audio_sample_rate int, -- null = video only
    audio_channels int, -- null = video only
    audio_bitrate int, -- null = video only
    audio_codec varchar(255), -- null = video only
    metadata jsonb NOT NULL DEFAULT '{}',
    -- Timestamps
    created_at timestamptz NOT NULL DEFAULT NOW()
);

CREATE TABLE stream_events (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    stream_id uuid NOT NULL, -- foreign key to streams(id)
    title varchar(255) NOT NULL,
    message text NOT NULL,
    level int NOT NULL, -- 0 = info, 1 = warning, 2 = error
    -- Timestamps
    created_at timestamptz NOT NULL DEFAULT NOW()
);

-- Indexes

CREATE INDEX users_username_idx ON users (username);

CREATE INDEX global_roles_user_id_idx ON global_role_grants (user_id);
CREATE INDEX global_roles_global_role_id_idx ON global_role_grants (global_role_id);
CREATE INDEX global_roles_rank_idx ON global_roles (rank);

CREATE INDEX channel_roles_user_id_idx ON channel_role_grants (user_id);
CREATE INDEX channel_roles_channel_role_id_idx ON channel_role_grants (channel_role_id);
CREATE INDEX channel_roles_rank_idx ON channel_roles (rank);

CREATE INDEX streams_channel_id_idx ON streams (channel_id);

CREATE INDEX stream_bitrate_updates_stream_id_idx ON stream_bitrate_updates (stream_id);
CREATE INDEX stream_bitrate_updates_created_at_idx ON stream_bitrate_updates (created_at);

CREATE INDEX stream_variants_stream_id_idx ON stream_variants (stream_id);

CREATE INDEX stream_events_stream_id_idx ON stream_events (stream_id);
-- CONSTRAINTS

ALTER TABLE IF EXISTS users ADD CONSTRAINT users_username_unique UNIQUE (username);

ALTER TABLE IF EXISTS global_roles ADD CONSTRAINT global_roles_name_unique UNIQUE (name);
ALTER TABLE IF EXISTS global_roles ADD CONSTRAINT global_roles_rank_unique UNIQUE (rank);

ALTER TABLE IF EXISTS channel_roles ADD CONSTRAINT channel_roles_name_unique UNIQUE (channel_id, name);
ALTER TABLE IF EXISTS channel_roles ADD CONSTRAINT channel_roles_rank_unique UNIQUE (channel_id, rank);

ALTER TABLE IF EXISTS stream_variants ADD CONSTRAINT stream_variants_name_unique UNIQUE (stream_id, name);
-- Foreign keys

ALTER TABLE sessions ADD CONSTRAINT sessions_user_id_fkey FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;

ALTER TABLE global_role_grants ADD CONSTRAINT global_role_grants_user_id_fkey FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;
ALTER TABLE global_role_grants ADD CONSTRAINT global_role_grants_global_role_id_fkey FOREIGN KEY (global_role_id) REFERENCES global_roles(id) ON DELETE CASCADE;

ALTER TABLE channel_role_grants ADD CONSTRAINT channel_role_grants_user_id_fkey FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;
ALTER TABLE channel_role_grants ADD CONSTRAINT channel_role_grants_channel_role_id_fkey FOREIGN KEY (channel_role_id) REFERENCES channel_roles(id) ON DELETE CASCADE;

ALTER TABLE streams ADD CONSTRAINT streams_channel_id_fkey FOREIGN KEY (channel_id) REFERENCES users(id) ON DELETE CASCADE;

ALTER TABLE stream_bitrate_updates ADD CONSTRAINT stream_bitrate_updates_stream_id_fkey FOREIGN KEY (stream_id) REFERENCES streams(id) ON DELETE CASCADE;

ALTER TABLE stream_variants ADD CONSTRAINT stream_variants_stream_id_fkey FOREIGN KEY (stream_id) REFERENCES streams(id) ON DELETE CASCADE;

ALTER TABLE stream_events ADD CONSTRAINT stream_events_stream_id_fkey FOREIGN KEY (stream_id) REFERENCES streams(id) ON DELETE CASCADE;
