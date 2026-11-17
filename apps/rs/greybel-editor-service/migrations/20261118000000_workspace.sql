-- Shared projects: a workspace with its entry file and files. The legacy "Code" table of greybel-editor-be stays
-- as it is and is only read.
CREATE TABLE workspace (
    id UUID PRIMARY KEY,
    entry TEXT NOT NULL,
    -- SHA-256 of the entry and files; saving the same project again returns the existing workspace.
    content_hash BYTEA NOT NULL UNIQUE,
    creation_time TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE workspace_file (
    workspace_id UUID NOT NULL REFERENCES workspace (id) ON DELETE CASCADE,
    path TEXT NOT NULL,
    content TEXT NOT NULL,
    PRIMARY KEY (workspace_id, path)
);
