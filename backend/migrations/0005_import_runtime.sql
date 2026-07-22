ALTER TABLE import_jobs ADD COLUMN worker_id TEXT;
ALTER TABLE import_jobs ADD COLUMN lease_expires_at TEXT;
ALTER TABLE import_jobs ADD COLUMN attempt_count INTEGER NOT NULL DEFAULT 0;
ALTER TABLE import_jobs ADD COLUMN cancel_requested_at TEXT;
ALTER TABLE import_jobs ADD COLUMN started_at TEXT;
ALTER TABLE import_jobs ADD COLUMN last_heartbeat_at TEXT;

CREATE TABLE IF NOT EXISTS import_job_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    job_id TEXT NOT NULL REFERENCES import_jobs(id) ON DELETE CASCADE,
    event_type TEXT NOT NULL,
    status TEXT NOT NULL,
    stage TEXT NOT NULL,
    progress INTEGER NOT NULL,
    message TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_import_jobs_worker_queue
    ON import_jobs(status, lease_expires_at, created_at);
CREATE INDEX IF NOT EXISTS idx_import_job_events_job_id
    ON import_job_events(job_id, id ASC);
