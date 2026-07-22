ALTER TABLE projects ADD COLUMN owner_name TEXT;

CREATE TABLE IF NOT EXISTS project_tags (
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    tag TEXT NOT NULL,
    sort_order INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY(project_id, tag)
);

CREATE INDEX IF NOT EXISTS idx_project_tags_project_id ON project_tags(project_id);
