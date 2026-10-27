-- Legacy numeric ports remain the CSV/JSON compatibility projection.
CREATE TABLE server_endpoints (
    id INTEGER PRIMARY KEY,
    server_id INTEGER NOT NULL REFERENCES server_records(id) ON DELETE CASCADE,
    port INTEGER NOT NULL CHECK(port BETWEEN 1 AND 65535),
    transport TEXT NOT NULL DEFAULT 'Unspecified' CHECK(transport IN ('Unspecified','TCP','UDP')),
    protocol TEXT NOT NULL DEFAULT '' CHECK(length(protocol) <= 40),
    purpose TEXT NOT NULL DEFAULT '' CHECK(length(purpose) <= 240),
    UNIQUE(server_id, port, transport)
);
INSERT INTO server_endpoints(server_id,port)
SELECT s.id, CAST(j.value AS INTEGER) FROM server_records s,
json_each('[' || replace(s.ports,' ','') || ']') j;

CREATE TRIGGER server_endpoint_insert AFTER INSERT ON server_records BEGIN
    INSERT INTO server_endpoints(server_id,port)
    SELECT NEW.id, CAST(value AS INTEGER)
    FROM json_each('[' || replace(NEW.ports,' ','') || ']');
END;
CREATE TRIGGER server_endpoint_sync AFTER UPDATE OF ports ON server_records BEGIN
    DELETE FROM server_endpoints WHERE server_id=NEW.id AND port NOT IN
    (SELECT CAST(value AS INTEGER) FROM json_each('[' || replace(NEW.ports,' ','') || ']'));
    INSERT INTO server_endpoints(server_id,port)
    SELECT NEW.id, CAST(j.value AS INTEGER)
    FROM json_each('[' || replace(NEW.ports,' ','') || ']') j
    WHERE NOT EXISTS (SELECT 1 FROM server_endpoints e
        WHERE e.server_id=NEW.id AND e.port=CAST(j.value AS INTEGER));
END;
