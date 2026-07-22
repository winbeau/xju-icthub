ALTER TABLE import_jobs ADD COLUMN agent_thread_id TEXT;
ALTER TABLE import_jobs ADD COLUMN agent_result_json TEXT;

CREATE TABLE IF NOT EXISTS agent_runs (
    id TEXT PRIMARY KEY NOT NULL,
    job_id TEXT NOT NULL REFERENCES import_jobs(id) ON DELETE CASCADE,
    runner TEXT NOT NULL,
    model TEXT NOT NULL,
    base_url_origin TEXT,
    status TEXT NOT NULL,
    input_sha256 TEXT NOT NULL,
    output_json TEXT,
    raw_events_path TEXT,
    error_message TEXT,
    started_at TEXT,
    completed_at TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_agent_runs_job_created
    ON agent_runs(job_id, created_at DESC);
