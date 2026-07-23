ALTER TABLE resources ADD COLUMN mime_type TEXT;
ALTER TABLE resources ADD COLUMN size_bytes INTEGER;
ALTER TABLE resources ADD COLUMN display_path TEXT;
ALTER TABLE resources ADD COLUMN preview_kind TEXT;
ALTER TABLE resources ADD COLUMN entry_path TEXT;
ALTER TABLE resources ADD COLUMN source_import_job_id TEXT;
ALTER TABLE resources ADD COLUMN source_artifact_id TEXT;
ALTER TABLE resources ADD COLUMN sha256 TEXT;

CREATE INDEX IF NOT EXISTS idx_resources_source_artifact
    ON resources(source_import_job_id, source_artifact_id);

CREATE TABLE IF NOT EXISTS project_import_commits (
    job_id TEXT PRIMARY KEY NOT NULL REFERENCES import_jobs(id) ON DELETE CASCADE,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    committed_by_sid TEXT NOT NULL,
    committed_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS resource_preview_tokens (
    token_hash TEXT PRIMARY KEY NOT NULL,
    resource_id TEXT NOT NULL REFERENCES resources(id) ON DELETE CASCADE,
    created_by_sid TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_resource_preview_tokens_expiry
    ON resource_preview_tokens(expires_at);
