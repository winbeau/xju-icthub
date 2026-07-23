CREATE TABLE IF NOT EXISTS import_artifact_preview_tokens (
    token_hash TEXT PRIMARY KEY NOT NULL,
    job_id TEXT NOT NULL REFERENCES import_jobs(id) ON DELETE CASCADE,
    artifact_id TEXT NOT NULL REFERENCES import_artifacts(id) ON DELETE CASCADE,
    created_by_sid TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_import_artifact_preview_tokens_expiry
    ON import_artifact_preview_tokens(expires_at);
