CREATE TABLE sites (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL CHECK (length(trim(name)) BETWEEN 1 AND 160),
    url TEXT NOT NULL CHECK (length(trim(url)) BETWEEN 8 AND 2048),
    environment TEXT NOT NULL DEFAULT 'production' CHECK (length(trim(environment)) BETWEEN 1 AND 64),
    status TEXT NOT NULL DEFAULT 'active' CHECK (length(trim(status)) BETWEEN 1 AND 64),
    purpose TEXT NOT NULL DEFAULT '' CHECK (length(purpose) <= 1024),
    tags TEXT NOT NULL DEFAULT '' CHECK (length(tags) <= 2048),
    position INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    archived_at TEXT
);

CREATE INDEX sites_active_idx ON sites(archived_at, position, id);
CREATE INDEX sites_name_idx ON sites(name COLLATE NOCASE, id);
