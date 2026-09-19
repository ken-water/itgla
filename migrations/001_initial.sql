CREATE TABLE projects (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL COLLATE NOCASE UNIQUE CHECK (length(trim(name)) BETWEEN 1 AND 120),
    description TEXT NOT NULL DEFAULT '',
    position INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    archived_at TEXT
);

CREATE TABLE assets (
    id INTEGER PRIMARY KEY,
    project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    kind TEXT NOT NULL CHECK (kind IN ('website', 'domain', 'certificate', 'server', 'service')),
    name TEXT NOT NULL CHECK (length(trim(name)) BETWEEN 1 AND 240),
    detail TEXT NOT NULL DEFAULT '',
    status_detail TEXT NOT NULL DEFAULT '',
    environment TEXT NOT NULL DEFAULT '',
    health TEXT NOT NULL DEFAULT 'healthy' CHECK (health IN ('healthy', 'warning', 'critical')),
    position INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    archived_at TEXT,
    UNIQUE(project_id, kind, name)
);

CREATE TABLE tags (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL COLLATE NOCASE UNIQUE CHECK (length(trim(name)) BETWEEN 1 AND 48),
    color TEXT NOT NULL DEFAULT '#5575b7'
);

CREATE TABLE asset_tags (
    asset_id INTEGER NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
    tag_id INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (asset_id, tag_id)
);

CREATE TABLE relationships (
    id INTEGER PRIMARY KEY,
    source_asset_id INTEGER NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
    target_asset_id INTEGER NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
    kind TEXT NOT NULL CHECK (kind IN ('deploys_to', 'uses_domain', 'protected_by', 'depends_on', 'serves')),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CHECK (source_asset_id <> target_asset_id),
    UNIQUE(source_asset_id, target_asset_id, kind)
);

CREATE INDEX assets_project_active_idx ON assets(project_id, archived_at, position);
CREATE INDEX relationships_source_idx ON relationships(source_asset_id);
CREATE INDEX relationships_target_idx ON relationships(target_asset_id);
