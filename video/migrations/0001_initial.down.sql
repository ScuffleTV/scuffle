-- Add down migration script here

DROP TABLE recording_renditions CASCADE;
DROP TABLE recordings CASCADE;
DROP TABLE organizations CASCADE;
DROP TABLE access_tokens CASCADE;
DROP TABLE s3_buckets CASCADE;
DROP TABLE transcoding_configs CASCADE;
DROP TABLE recording_configs CASCADE;
DROP TABLE rooms CASCADE;
DROP TABLE playback_key_pairs CASCADE;
DROP TABLE playback_sessions CASCADE;
DROP TYPE rendition_video;
DROP TYPE rendition_audio;
DROP TYPE playback_session_device;
DROP TYPE playback_session_platform;
DROP TYPE playback_session_browser;
DROP TYPE room_status;
