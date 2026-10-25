-- mocktail initial SQLite schema.
-- Applied automatically at startup via embedded migrations or with:
--   wrangler d1 migrations apply mocktail

CREATE TABLE IF NOT EXISTS workspaces (
    id TEXT PRIMARY KEY,
    slug TEXT UNIQUE NOT NULL,
    name TEXT NOT NULL,
    created_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS collections (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL,
    slug TEXT NOT NULL,
    name TEXT NOT NULL,
    description TEXT NOT NULL,
    source TEXT NOT NULL,
    base_path TEXT NOT NULL,
    cors TEXT NOT NULL, -- JSON
    version INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    UNIQUE (workspace_id, slug)
);
CREATE INDEX IF NOT EXISTS idx_collections_ws ON collections (workspace_id);

CREATE TABLE IF NOT EXISTS folders (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL,
    collection_id TEXT NOT NULL,
    name TEXT NOT NULL,
    parent_id TEXT,
    created_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_folders_ws_coll ON folders (
    workspace_id, collection_id
);
CREATE INDEX IF NOT EXISTS idx_folders_coll ON folders (collection_id);

CREATE TABLE IF NOT EXISTS mocks (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL,
    collection_id TEXT NOT NULL,
    folder_id TEXT,
    method TEXT NOT NULL,
    path_template TEXT NOT NULL,
    matchers TEXT NOT NULL, -- JSON
    response TEXT NOT NULL, -- JSON
    priority INTEGER NOT NULL,
    delay_ms INTEGER NOT NULL,
    fault TEXT,
    rate_limit TEXT, -- JSON
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_mocks_ws_coll ON mocks (
    workspace_id, collection_id
);
CREATE INDEX IF NOT EXISTS idx_mocks_coll ON mocks (collection_id);

CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,
    username TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL,
    created_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS user_identities (
    provider TEXT NOT NULL,
    user_id TEXT NOT NULL,
    email TEXT,
    subject TEXT NOT NULL,
    PRIMARY KEY (provider, subject)
);
CREATE INDEX IF NOT EXISTS idx_identities_user ON user_identities (user_id);

CREATE TABLE IF NOT EXISTS memberships (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    workspace_id TEXT NOT NULL,
    role TEXT NOT NULL,
    UNIQUE (user_id, workspace_id)
);
CREATE INDEX IF NOT EXISTS idx_memberships_ws ON memberships (workspace_id);

CREATE TABLE IF NOT EXISTS request_logs (
    id TEXT PRIMARY KEY,
    request_id TEXT NOT NULL,
    workspace_id TEXT NOT NULL,
    collection_id TEXT,
    received_at INTEGER NOT NULL,
    method TEXT NOT NULL,
    path TEXT NOT NULL DEFAULT '',
    requestheaders TEXT NOT NULL, -- JSON
    requestbody TEXT NOT NULL,
    matchedmockid TEXT,
    matchreason TEXT,
    status INTEGER NOT NULL DEFAULT 0,
    response TEXT NOT NULL, -- JSON
    ip TEXT,
    country TEXT,
    lat REAL,
    lng REAL
);
CREATE INDEX IF NOT EXISTS idx_logs_ws_time ON request_logs (
    workspace_id, received_at, id
);
CREATE INDEX IF NOT EXISTS idx_logs_req ON request_logs (
    workspace_id, request_id
);
