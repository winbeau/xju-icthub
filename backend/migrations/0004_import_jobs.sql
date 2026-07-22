CREATE TABLE IF NOT EXISTS import_jobs (
    id TEXT PRIMARY KEY NOT NULL,
    status TEXT NOT NULL,
    stage TEXT NOT NULL,
    progress INTEGER NOT NULL DEFAULT 0,
    source_kind TEXT NOT NULL,
    source_name TEXT NOT NULL,
    analysis_engine TEXT NOT NULL DEFAULT 'deterministic_fallback',
    result_json TEXT,
    error_message TEXT,
    created_by_sid TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    completed_at TEXT
);

CREATE TABLE IF NOT EXISTS import_inputs (
    id TEXT PRIMARY KEY NOT NULL,
    job_id TEXT NOT NULL REFERENCES import_jobs(id) ON DELETE CASCADE,
    input_kind TEXT NOT NULL,
    provider TEXT NOT NULL,
    display_name TEXT NOT NULL,
    source_ref TEXT,
    local_path TEXT,
    mime_type TEXT,
    size_bytes INTEGER,
    sha256 TEXT,
    sort_order INTEGER NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'pending',
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS import_artifacts (
    id TEXT PRIMARY KEY NOT NULL,
    job_id TEXT NOT NULL REFERENCES import_jobs(id) ON DELETE CASCADE,
    input_id TEXT REFERENCES import_inputs(id) ON DELETE SET NULL,
    relative_path TEXT NOT NULL,
    artifact_kind TEXT NOT NULL,
    mime_type TEXT,
    size_bytes INTEGER NOT NULL DEFAULT 0,
    extractor TEXT NOT NULL DEFAULT 'file_index',
    metadata_json TEXT NOT NULL DEFAULT '{}',
    is_cover_candidate INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_import_jobs_creator_created
    ON import_jobs(created_by_sid, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_import_inputs_job_sort
    ON import_inputs(job_id, sort_order ASC);
CREATE INDEX IF NOT EXISTS idx_import_artifacts_job_kind
    ON import_artifacts(job_id, artifact_kind, relative_path);
