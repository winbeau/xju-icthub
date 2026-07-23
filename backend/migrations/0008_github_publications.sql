CREATE TABLE IF NOT EXISTS github_repo_sequences (
    owner TEXT PRIMARY KEY NOT NULL,
    next_number INTEGER NOT NULL DEFAULT 1,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS github_publications (
    id TEXT PRIMARY KEY NOT NULL,
    job_id TEXT NOT NULL UNIQUE REFERENCES import_jobs(id) ON DELETE CASCADE,
    requested_by_sid TEXT NOT NULL,
    owner TEXT NOT NULL,
    repo_number INTEGER NOT NULL,
    repo_name TEXT NOT NULL,
    repo_url TEXT,
    source_ref TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'queued',
    error_message TEXT,
    commit_sha TEXT,
    worker_id TEXT,
    lease_expires_at TEXT,
    attempt_count INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    started_at TEXT,
    completed_at TEXT,
    UNIQUE(owner, repo_number),
    UNIQUE(owner, repo_name)
);

CREATE INDEX IF NOT EXISTS idx_github_publications_queue
    ON github_publications(status, lease_expires_at, created_at);
