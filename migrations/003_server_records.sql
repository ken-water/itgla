CREATE TABLE server_records (
    id INTEGER PRIMARY KEY,
    purpose TEXT NOT NULL CHECK (length(trim(purpose)) BETWEEN 1 AND 240),
    ip_address TEXT NOT NULL CHECK (length(trim(ip_address)) BETWEEN 2 AND 45),
    ports TEXT NOT NULL CHECK (length(trim(ports)) BETWEEN 1 AND 320),
    position INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    archived_at TEXT
);

CREATE INDEX server_records_active_idx
    ON server_records(archived_at, position, id);
