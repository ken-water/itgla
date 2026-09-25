CREATE TABLE server_custom_columns (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL COLLATE NOCASE UNIQUE
        CHECK (length(trim(name)) BETWEEN 1 AND 64),
    position INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE server_custom_values (
    server_id INTEGER NOT NULL REFERENCES server_records(id) ON DELETE CASCADE,
    column_id INTEGER NOT NULL REFERENCES server_custom_columns(id) ON DELETE CASCADE,
    value TEXT NOT NULL DEFAULT '' CHECK (length(value) <= 2048),
    PRIMARY KEY (server_id, column_id)
);

CREATE INDEX server_custom_values_column_idx
    ON server_custom_values(column_id, server_id);
