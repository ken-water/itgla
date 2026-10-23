CREATE TABLE operational_inventory (
    id INTEGER PRIMARY KEY,
    kind INTEGER NOT NULL CHECK(kind IN (0,1)),
    data TEXT NOT NULL CHECK(length(data) <= 32768),
    credentials BLOB,
    hidden INTEGER NOT NULL DEFAULT 0 CHECK(hidden IN (0,1))
);
CREATE TABLE operational_hosts (
    inventory_id INTEGER NOT NULL REFERENCES operational_inventory(id) ON DELETE CASCADE,
    server_id INTEGER NOT NULL REFERENCES server_records(id),
    PRIMARY KEY(inventory_id,server_id)
);
CREATE INDEX operational_hosts_server ON operational_hosts(server_id);
CREATE TABLE operational_relations (
    id INTEGER PRIMARY KEY,
    source_id INTEGER NOT NULL REFERENCES operational_inventory(id) ON DELETE CASCADE,
    target_id INTEGER NOT NULL REFERENCES operational_inventory(id) ON DELETE CASCADE,
    kind INTEGER NOT NULL CHECK(kind BETWEEN 0 AND 3),
    notes TEXT NOT NULL DEFAULT '' CHECK(length(notes) <= 2048),
    CHECK(source_id <> target_id),
    UNIQUE(source_id,target_id,kind)
);
