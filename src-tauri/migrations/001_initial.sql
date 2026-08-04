PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS schema_migrations (
  version INTEGER PRIMARY KEY,
  applied_at_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS users (
  id TEXT PRIMARY KEY,
  email TEXT NOT NULL COLLATE NOCASE UNIQUE,
  display_name TEXT NOT NULL CHECK(length(display_name) BETWEEN 1 AND 120),
  roles_json TEXT NOT NULL CHECK(json_valid(roles_json)),
  device_ids_json TEXT NOT NULL CHECK(json_valid(device_ids_json)),
  status TEXT NOT NULL CHECK(status IN ('active', 'suspended', 'expired')),
  last_authenticated_at_ms INTEGER
);

CREATE TABLE IF NOT EXISTS launch_policies (
  id TEXT PRIMARY KEY,
  title TEXT NOT NULL CHECK(length(title) BETWEEN 1 AND 160),
  purpose TEXT NOT NULL CHECK(length(purpose) BETWEEN 20 AND 2000),
  owner_user_id TEXT NOT NULL REFERENCES users(id),
  status TEXT NOT NULL CHECK(status IN ('draft', 'approved', 'active', 'stopped', 'expired')),
  environment_id TEXT NOT NULL,
  approved_device_ids_json TEXT NOT NULL CHECK(json_valid(approved_device_ids_json)),
  adversarial_approved INTEGER NOT NULL DEFAULT 0 CHECK(adversarial_approved IN (0, 1)),
  retention_days INTEGER NOT NULL CHECK(retention_days BETWEEN 1 AND 30),
  starts_at_ms INTEGER,
  expires_at_ms INTEGER,
  approved_by TEXT REFERENCES users(id),
  approved_at_ms INTEGER,
  CHECK(expires_at_ms IS NULL OR starts_at_ms IS NULL OR expires_at_ms > starts_at_ms)
);

CREATE INDEX IF NOT EXISTS launch_policies_status_expiry_idx ON launch_policies(status, expires_at_ms);

CREATE TABLE IF NOT EXISTS participant_consents (
  id TEXT PRIMARY KEY,
  launch_policy_id TEXT NOT NULL REFERENCES launch_policies(id),
  participant_label TEXT NOT NULL,
  consent_artifact_id TEXT NOT NULL,
  scope_json TEXT NOT NULL CHECK(json_valid(scope_json)),
  signed_at_ms INTEGER NOT NULL,
  revoked_at_ms INTEGER
);

CREATE TABLE IF NOT EXISTS model_configurations (
  id TEXT PRIMARY KEY,
  response_model_id TEXT NOT NULL,
  transcription_model_id TEXT NOT NULL,
  translation_language TEXT NOT NULL,
  answer_depth TEXT NOT NULL CHECK(answer_depth IN ('brief', 'balanced', 'detailed')),
  question_confidence_threshold REAL NOT NULL CHECK(question_confidence_threshold BETWEEN 0 AND 1),
  processing_boundary_id TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS ai_profiles (
  id TEXT PRIMARY KEY,
  owner_user_id TEXT NOT NULL REFERENCES users(id),
  name TEXT NOT NULL CHECK(length(name) BETWEEN 1 AND 100),
  status TEXT NOT NULL CHECK(status IN ('draft', 'ready', 'archived')),
  manual_context TEXT NOT NULL DEFAULT '',
  model_configuration_id TEXT REFERENCES model_configurations(id),
  created_at_ms INTEGER NOT NULL,
  updated_at_ms INTEGER NOT NULL,
  revision INTEGER NOT NULL DEFAULT 1 CHECK(revision > 0),
  UNIQUE(owner_user_id, name)
);

CREATE TABLE IF NOT EXISTS vacancy_sources (
  id TEXT PRIMARY KEY,
  profile_id TEXT NOT NULL UNIQUE REFERENCES ai_profiles(id) ON DELETE CASCADE,
  source_kind TEXT NOT NULL CHECK(source_kind IN ('url', 'pasted_text')),
  source_value TEXT NOT NULL,
  role_title TEXT NOT NULL DEFAULT '',
  company_context TEXT NOT NULL DEFAULT '',
  responsibilities_json TEXT NOT NULL DEFAULT '[]' CHECK(json_valid(responsibilities_json)),
  requirements_json TEXT NOT NULL DEFAULT '[]' CHECK(json_valid(requirements_json)),
  review_status TEXT NOT NULL CHECK(review_status IN ('pending', 'needs_review', 'confirmed', 'rejected')),
  provenance_json TEXT NOT NULL DEFAULT '{}' CHECK(json_valid(provenance_json))
);

CREATE TABLE IF NOT EXISTS profile_sources (
  id TEXT PRIMARY KEY,
  profile_id TEXT NOT NULL REFERENCES ai_profiles(id) ON DELETE CASCADE,
  kind TEXT NOT NULL CHECK(kind IN ('resume', 'manual', 'project')),
  display_name TEXT NOT NULL,
  mime_type TEXT,
  storage_key TEXT,
  extracted_facts_json TEXT NOT NULL DEFAULT '[]' CHECK(json_valid(extracted_facts_json)),
  content_status TEXT NOT NULL CHECK(content_status IN ('pending', 'allowed', 'redacted', 'rejected')),
  redaction_summary TEXT,
  checksum TEXT
);

CREATE TABLE IF NOT EXISTS meetings (
  id TEXT PRIMARY KEY,
  launch_policy_id TEXT NOT NULL REFERENCES launch_policies(id),
  profile_id TEXT NOT NULL REFERENCES ai_profiles(id),
  profile_revision INTEGER NOT NULL CHECK(profile_revision > 0),
  model_snapshot_json TEXT NOT NULL CHECK(json_valid(model_snapshot_json)),
  title TEXT NOT NULL CHECK(length(title) BETWEEN 1 AND 160),
  status TEXT NOT NULL CHECK(status IN ('prepared', 'gating', 'running', 'stopping', 'completed', 'failed', 'expired')),
  mode TEXT NOT NULL CHECK(mode IN ('standard_lab', 'adversarial_lab')),
  capture_configuration_id TEXT NOT NULL,
  context_generation INTEGER NOT NULL DEFAULT 0 CHECK(context_generation >= 0),
  created_at_ms INTEGER NOT NULL,
  started_at_ms INTEGER,
  ended_at_ms INTEGER,
  retention_expires_at_ms INTEGER NOT NULL,
  failure_code TEXT
);

CREATE INDEX IF NOT EXISTS meetings_policy_profile_status_date_idx
  ON meetings(launch_policy_id, profile_id, status, created_at_ms DESC, id DESC);

CREATE TABLE IF NOT EXISTS capture_configurations (
  id TEXT PRIMARY KEY,
  meeting_id TEXT NOT NULL UNIQUE REFERENCES meetings(id) ON DELETE CASCADE,
  display_id INTEGER NOT NULL,
  area_json TEXT CHECK(area_json IS NULL OR json_valid(area_json)),
  backing_scale REAL NOT NULL CHECK(backing_scale > 0),
  capture_system_audio INTEGER NOT NULL CHECK(capture_system_audio IN (0, 1)),
  capture_microphone INTEGER NOT NULL CHECK(capture_microphone IN (0, 1)),
  shows_cursor_in_own_artifacts INTEGER NOT NULL DEFAULT 0 CHECK(shows_cursor_in_own_artifacts IN (0, 1)),
  auto_screenshot_mode TEXT NOT NULL CHECK(auto_screenshot_mode IN ('off', 'display', 'area')),
  sound_threshold REAL NOT NULL CHECK(sound_threshold BETWEEN 0 AND 1),
  matrix_row_id TEXT
);

CREATE TABLE IF NOT EXISTS artifacts (
  id TEXT PRIMARY KEY,
  meeting_id TEXT REFERENCES meetings(id) ON DELETE CASCADE,
  kind TEXT NOT NULL CHECK(kind IN ('audio_chunk', 'recording', 'screenshot', 'consent', 'profile_file', 'export_bundle')),
  storage_key TEXT NOT NULL UNIQUE,
  mime_type TEXT NOT NULL,
  byte_length INTEGER NOT NULL CHECK(byte_length >= 0),
  checksum TEXT NOT NULL,
  content_status TEXT NOT NULL CHECK(content_status IN ('pending', 'allowed', 'redacted', 'rejected', 'expired', 'deleted')),
  redacted_from_artifact_id TEXT REFERENCES artifacts(id),
  created_at_ms INTEGER NOT NULL,
  expires_at_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS artifacts_meeting_status_expiry_idx ON artifacts(meeting_id, content_status, expires_at_ms);

CREATE TABLE IF NOT EXISTS transcript_segments (
  id TEXT PRIMARY KEY,
  meeting_id TEXT NOT NULL REFERENCES meetings(id) ON DELETE CASCADE,
  sequence INTEGER NOT NULL CHECK(sequence >= 0),
  speaker TEXT NOT NULL CHECK(speaker IN ('interviewer', 'user', 'unknown')),
  text TEXT NOT NULL,
  confidence REAL NOT NULL CHECK(confidence BETWEEN 0 AND 1),
  is_final INTEGER NOT NULL CHECK(is_final IN (0, 1)),
  is_question INTEGER NOT NULL CHECK(is_question IN (0, 1)),
  started_at_ms INTEGER NOT NULL,
  ended_at_ms INTEGER NOT NULL,
  UNIQUE(meeting_id, sequence),
  CHECK(ended_at_ms >= started_at_ms)
);

CREATE TABLE IF NOT EXISTS chat_threads (
  id TEXT PRIMARY KEY,
  meeting_id TEXT NOT NULL REFERENCES meetings(id) ON DELETE CASCADE,
  kind TEXT NOT NULL CHECK(kind IN ('live', 'side')),
  created_at_ms INTEGER NOT NULL,
  UNIQUE(meeting_id, kind)
);

CREATE TABLE IF NOT EXISTS chat_messages (
  id TEXT PRIMARY KEY,
  thread_id TEXT NOT NULL REFERENCES chat_threads(id) ON DELETE CASCADE,
  sequence INTEGER NOT NULL CHECK(sequence >= 0),
  role TEXT NOT NULL CHECK(role IN ('user', 'assistant', 'system')),
  content TEXT NOT NULL,
  status TEXT NOT NULL CHECK(status IN ('pending', 'streaming', 'complete', 'error', 'cancelled')),
  reply_to_segment_id TEXT REFERENCES transcript_segments(id),
  artifact_ids_json TEXT NOT NULL DEFAULT '[]' CHECK(json_valid(artifact_ids_json)),
  profile_source_ids_json TEXT NOT NULL DEFAULT '[]' CHECK(json_valid(profile_source_ids_json)),
  context_generation INTEGER NOT NULL CHECK(context_generation >= 0),
  confidence REAL CHECK(confidence IS NULL OR confidence BETWEEN 0 AND 1),
  created_at_ms INTEGER NOT NULL,
  UNIQUE(thread_id, sequence)
);

CREATE TABLE IF NOT EXISTS diagram_documents (
  id TEXT PRIMARY KEY,
  meeting_id TEXT NOT NULL UNIQUE REFERENCES meetings(id) ON DELETE CASCADE,
  revision INTEGER NOT NULL DEFAULT 1 CHECK(revision > 0),
  nodes_json TEXT NOT NULL DEFAULT '[]' CHECK(json_valid(nodes_json)),
  edges_json TEXT NOT NULL DEFAULT '[]' CHECK(json_valid(edges_json)),
  pending_proposal_json TEXT CHECK(pending_proposal_json IS NULL OR json_valid(pending_proposal_json))
);

CREATE TABLE IF NOT EXISTS app_preferences (
  user_id TEXT PRIMARY KEY REFERENCES users(id),
  theme TEXT NOT NULL CHECK(theme IN ('light', 'dark', 'auto')),
  overlay_position_by_display_json TEXT NOT NULL DEFAULT '{}' CHECK(json_valid(overlay_position_by_display_json)),
  selected_display_id INTEGER,
  hotkeys_json TEXT NOT NULL DEFAULT '{}' CHECK(json_valid(hotkeys_json)),
  reduce_visual_effects_override TEXT NOT NULL CHECK(reduce_visual_effects_override IN ('system', 'reduce')),
  updated_at_ms INTEGER NOT NULL,
  revision INTEGER NOT NULL DEFAULT 1 CHECK(revision > 0)
);

CREATE TABLE IF NOT EXISTS mock_entitlements (
  user_id TEXT PRIMARY KEY REFERENCES users(id),
  plan TEXT NOT NULL DEFAULT 'demo',
  status TEXT NOT NULL CHECK(status IN ('inactive', 'active', 'expired')),
  source TEXT NOT NULL DEFAULT 'mock_button' CHECK(source = 'mock_button'),
  granted_at_ms INTEGER,
  expires_at_ms INTEGER
);

CREATE TABLE IF NOT EXISTS audit_events (
  sequence INTEGER PRIMARY KEY AUTOINCREMENT,
  id TEXT NOT NULL UNIQUE,
  occurred_at_ms INTEGER NOT NULL,
  user_id TEXT REFERENCES users(id),
  launch_policy_id TEXT REFERENCES launch_policies(id),
  meeting_id TEXT REFERENCES meetings(id),
  action TEXT NOT NULL,
  outcome TEXT NOT NULL CHECK(outcome IN ('allowed', 'denied', 'succeeded', 'failed', 'stopped')),
  reason_code TEXT NOT NULL,
  metadata_json TEXT NOT NULL DEFAULT '{}' CHECK(json_valid(metadata_json)),
  previous_hash TEXT NOT NULL,
  event_hash TEXT NOT NULL,
  retention_expires_at_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS audit_policy_time_action_idx ON audit_events(launch_policy_id, occurred_at_ms, action);

CREATE VIRTUAL TABLE IF NOT EXISTS meeting_search USING fts5(
  meeting_id UNINDEXED,
  owner_user_id UNINDEXED,
  created_at_ms UNINDEXED,
  title,
  vacancy,
  transcript,
  chat,
  tokenize = 'unicode61 remove_diacritics 2'
);

INSERT OR IGNORE INTO schema_migrations(version, applied_at_ms)
VALUES (1, CAST(unixepoch('subsec') * 1000 AS INTEGER));
