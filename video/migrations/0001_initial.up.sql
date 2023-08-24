-- Table Definitions

CREATE TYPE rendition AS ENUM ('VIDEO_SOURCE', 'VIDEO_HD', 'VIDEO_SD', 'VIDEO_LD', 'AUDIO_SOURCE');
CREATE TYPE playback_session_device AS ENUM ('UNKNOWN');
CREATE TYPE playback_session_platform AS ENUM ('UNKNOWN');
CREATE TYPE playback_session_browser AS ENUM ('UNKNOWN');
CREATE TYPE room_status AS ENUM ('OFFLINE', 'WAITING_FOR_TRANSCODER', 'READY');

CREATE TABLE organizations (
    id UUID NOT NULL PRIMARY KEY,
    name VARCHAR(32) NOT NULL,
    updated_at TIMESTAMPTZ(3) NOT NULL DEFAULT NOW(),
    tags JSONB NOT NULL DEFAULT '{}'::JSONB
);

CREATE TABLE access_tokens (
    id UUID NOT NULL PRIMARY KEY,
    organization_id UUID NOT NULL,

    version INT NOT NULL DEFAULT 0,
    last_access_at TIMESTAMPTZ(3),
    updated_at TIMESTAMPTZ(3) NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ(3),
    scopes bytes[] NOT NULL,
    tags JSONB NOT NULL DEFAULT '{}'::JSONB
);

CREATE TABLE s3_buckets (
    id UUID NOT NULL PRIMARY KEY,
    organization_id UUID NOT NULL,

    name VARCHAR(64) NOT NULL,
    region VARCHAR(64),
    endpoint VARCHAR(256),
    access_key VARCHAR(256) NOT NULL,
    secret_key VARCHAR(256) NOT NULL,
    public_url VARCHAR(256) NOT NULL,

    tags JSONB NOT NULL DEFAULT '{}'::JSONB
);

CREATE TABLE transcoding_configs (
    id UUID NOT NULL PRIMARY KEY,
    organization_id UUID NOT NULL,

    renditions rendition[] NOT NULL DEFAULT ARRAY['VIDEO_SOURCE', 'AUDIO_SOURCE'],
    updated_at TIMESTAMPTZ(3) NOT NULL DEFAULT NOW(),
    tags JSONB NOT NULL DEFAULT '{}'::JSONB
);

CREATE TABLE recording_configs (
    id UUID NOT NULL PRIMARY KEY,
    organization_id UUID NOT NULL,

    renditions rendition[] NOT NULL DEFAULT ARRAY['VIDEO_SOURCE', 'VIDEO_HD', 'VIDEO_SD', 'VIDEO_LD', 'AUDIO_SOURCE'],
    lifecycle_policies bytes[] NOT NULL DEFAULT ARRAY[],
    updated_at TIMESTAMPTZ(3) NOT NULL DEFAULT NOW(),
    s3_bucket_id UUID NOT NULL,

    tags JSONB NOT NULL DEFAULT '{}'::JSONB
);

CREATE TABLE rooms (
    id UUID NOT NULL PRIMARY KEY,
    organization_id UUID NOT NULL,

    transcoding_config_id UUID,
    recording_config_id UUID,

    private BOOLEAN NOT NULL DEFAULT FALSE,

    stream_key CHAR(32) NOT NULL,
    updated_at TIMESTAMPTZ(3) NOT NULL DEFAULT NOW(),

    last_live_at TIMESTAMPTZ(3),
    last_disconnected_at TIMESTAMPTZ(3),

    -- Room Session Stuff 

    status room_status NOT NULL DEFAULT 'OFFLINE',

    video_input bytes,
    audio_input bytes,

    active_ingest_connection_id UUID,
    active_recording_config bytes,
    active_transcoding_config bytes,
    active_recording_id UUID,

    ingest_bitrate INT,

    video_output bytes[],
    audio_output bytes[],
    tags JSONB NOT NULL DEFAULT '{}'::JSONB
);

CREATE TABLE recordings (
    id UUID NOT NULL PRIMARY KEY,
    organization_id UUID NOT NULL,

    room_id UUID,
    recording_config_id UUID,

    public BOOLEAN NOT NULL DEFAULT FALSE,
    deleted BOOLEAN NOT NULL DEFAULT FALSE,
    allow_dvr BOOLEAN NOT NULL DEFAULT FALSE,
    updated_at TIMESTAMPTZ(3) NOT NULL DEFAULT NOW()
);

CREATE TABLE recording_renditions (
    recording_id UUID NOT NULL,
    rendition rendition,

    organization_id UUID NOT NULL,
    segment_ids UUID[] NOT NULL,
    segment_durations INT4[] NOT NULL,
    timescale INT4 NOT NULL,
    size_bytes BIGINT NOT NULL,
    s3_bucket_id UUID NOT NULL,

    PRIMARY KEY (recording_id, rendition)
);

CREATE TABLE playback_key_pairs (
    id UUID PRIMARY KEY,
    organization_id UUID NOT NULL,

    public_key bytes NOT NULL,
    fingerprint VARCHAR(64) NOT NULL,
    created_at TIMESTAMPTZ(3) NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ(3) NOT NULL DEFAULT NOW(),
    tags JSONB NOT NULL DEFAULT '{}'::JSONB
);

CREATE TABLE playback_sessions (
    id UUID PRIMARY KEY,
    organization_id UUID NOT NULL,

    room_id UUID,
    recording_id UUID,

    user_id VARCHAR(128),
    playback_key_pair_id UUID,
    issued_at TIMESTAMPTZ(3),
    expires_at TIMESTAMPTZ(3) NOT NULL DEFAULT NOW() + INTERVAL '10 minutes',
    ip_address INET NOT NULL,
    user_agent VARCHAR(256),
    referer VARCHAR(256),
    origin VARCHAR(256),
    device playback_session_device NOT NULL DEFAULT 'UNKNOWN',
    platform playback_session_platform NOT NULL DEFAULT 'UNKNOWN',
    browser playback_session_browser NOT NULL DEFAULT 'UNKNOWN',
    player_version VARCHAR(32)
) WITH (ttl_expiration_expression = 'expires_at');

-- Relations

ALTER TABLE access_tokens ADD CONSTRAINT access_tokens_organization_id_fkey FOREIGN KEY (organization_id) REFERENCES organizations (id) ON DELETE CASCADE;

ALTER TABLE s3_buckets ADD CONSTRAINT s3_buckets_organization_id_fkey FOREIGN KEY (organization_id) REFERENCES organizations (id) ON DELETE CASCADE;

ALTER TABLE transcoding_configs ADD CONSTRAINT transcoding_configs_organization_id_fkey FOREIGN KEY (organization_id) REFERENCES organizations (id) ON DELETE CASCADE;

ALTER TABLE recording_configs ADD CONSTRAINT recording_configs_organization_id_fkey FOREIGN KEY (organization_id) REFERENCES organizations (id) ON DELETE CASCADE;
ALTER TABLE recording_configs ADD CONSTRAINT recording_configs_s3_bucket_id_fkey FOREIGN KEY (s3_bucket_id) REFERENCES s3_buckets (id) ON DELETE CASCADE;

ALTER TABLE rooms ADD CONSTRAINT rooms_organization_id_fkey FOREIGN KEY (organization_id) REFERENCES organizations (id) ON DELETE CASCADE;
ALTER TABLE rooms ADD CONSTRAINT rooms_transcoding_config_id_fkey FOREIGN KEY (transcoding_config_id) REFERENCES transcoding_configs (id) ON DELETE SET NULL;
ALTER TABLE rooms ADD CONSTRAINT rooms_recording_config_id_fkey FOREIGN KEY (recording_config_id) REFERENCES recording_configs (id) ON DELETE SET NULL;

ALTER TABLE recordings ADD CONSTRAINT recordings_organization_id_fkey FOREIGN KEY (organization_id) REFERENCES organizations (id) ON DELETE CASCADE;
ALTER TABLE recordings ADD CONSTRAINT recordings_room_id_fkey FOREIGN KEY (room_id) REFERENCES rooms (id) ON DELETE SET NULL;
ALTER TABLE recordings ADD CONSTRAINT recordings_recording_config_id_fkey FOREIGN KEY (recording_config_id) REFERENCES recording_configs (id) ON DELETE SET NULL;

ALTER TABLE recording_renditions ADD CONSTRAINT recording_renditions_organization_id_fkey FOREIGN KEY (organization_id) REFERENCES organizations (id);
ALTER TABLE recording_renditions ADD CONSTRAINT recording_renditions_recording_id_fkey FOREIGN KEY (recording_id) REFERENCES recordings (id);
ALTER TABLE recording_renditions ADD CONSTRAINT recording_renditions_s3_bucket_id_fkey FOREIGN KEY (s3_bucket_id) REFERENCES s3_buckets (id);

ALTER TABLE playback_key_pairs ADD CONSTRAINT playback_key_pairs_organization_id_fkey FOREIGN KEY (organization_id) REFERENCES organizations (id) ON DELETE CASCADE;

ALTER TABLE playback_sessions ADD CONSTRAINT playback_sessions_organization_id_fkey FOREIGN KEY (organization_id) REFERENCES organizations (id) ON DELETE CASCADE;
ALTER TABLE playback_sessions ADD CONSTRAINT playback_sessions_room_id_fkey FOREIGN KEY (room_id) REFERENCES rooms (id) ON DELETE CASCADE;
ALTER TABLE playback_sessions ADD CONSTRAINT playback_sessions_recording_id_fkey FOREIGN KEY (recording_id) REFERENCES recordings (id) ON DELETE CASCADE;
ALTER TABLE playback_sessions ADD CONSTRAINT playback_sessions_playback_key_pair_id_fkey FOREIGN KEY (playback_key_pair_id) REFERENCES playback_key_pairs (id) ON DELETE CASCADE;

-- Indexes

CREATE INDEX access_tokens_organization_id_idx ON access_tokens (organization_id);
CREATE INVERTED INDEX access_tokens_tags ON access_tokens (tags);

CREATE INDEX s3_buckets_organization_id_idx ON s3_buckets (organization_id);
CREATE INVERTED INDEX s3_buckets_tags ON s3_buckets (tags);

CREATE INDEX transcoding_configs_organization_id_idx ON transcoding_configs (organization_id);
CREATE INVERTED INDEX transcoding_configs_tags ON transcoding_configs (tags);

CREATE INDEX recording_configs_organization_id_idx ON recording_configs (organization_id);
CREATE INDEX recording_configs_s3_bucket_id_idx ON recording_configs (s3_bucket_id);
CREATE INVERTED INDEX recording_configs_tags ON recording_configs (tags);

CREATE INDEX rooms_organization_id_idx ON rooms (organization_id);
CREATE INDEX rooms_transcoding_config_id_idx ON rooms (transcoding_config_id);
CREATE INDEX rooms_recording_config_id_idx ON rooms (recording_config_id);
CREATE INVERTED INDEX rooms_tags ON rooms (tags);

CREATE INDEX recordings_organization_id_idx ON recordings (organization_id);
CREATE INDEX recordings_room_id_idx ON recordings (room_id);
CREATE INDEX recordings_recording_config_id_idx ON recordings (recording_config_id);

CREATE INDEX recording_renditions_organization_id_idx ON recording_renditions (organization_id);
CREATE INDEX recording_renditions_s3_bucket_id_idx ON recording_renditions (s3_bucket_id);

CREATE INDEX playback_key_pairs_organization_id_idx ON playback_key_pairs (organization_id);
CREATE INVERTED INDEX playback_key_pairs_tags ON playback_key_pairs (tags);

CREATE INDEX playback_sessions_organization_id_idx ON playback_sessions (organization_id);
CREATE INDEX playback_sessions_room_id_idx ON playback_sessions (room_id);
CREATE INDEX playback_sessions_recording_id_idx ON playback_sessions (recording_id);
CREATE INDEX playback_sessions_playback_key_pair_id_idx ON playback_sessions (playback_key_pair_id);
CREATE INDEX playback_sessions_user_id ON playback_sessions (user_id);
CREATE INDEX playback_sessions_ip_address ON playback_sessions (ip_address);
CREATE INDEX playback_sessions_expires_at ON playback_sessions (expires_at);
