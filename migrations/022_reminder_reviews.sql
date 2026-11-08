-- An expiry review is a manual acknowledgement, never a renewal or health signal.
CREATE TABLE IF NOT EXISTS reminder_reviews (
    resource_type TEXT NOT NULL CHECK(resource_type='domain'),
    resource_id INTEGER NOT NULL,
    source_date TEXT NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('reviewed','postponed')),
    snoozed_until TEXT NOT NULL DEFAULT '',
    reviewed_at TEXT,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY(resource_type,resource_id,source_date)
);
CREATE TABLE IF NOT EXISTS reminder_activity (
    id INTEGER PRIMARY KEY,
    resource_id INTEGER NOT NULL,
    source_date TEXT NOT NULL,
    actor TEXT NOT NULL DEFAULT 'Local workspace user',
    action TEXT NOT NULL CHECK(action IN ('reviewed','postponed','reopened')),
    occurred_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS reminder_activity_resource_idx ON reminder_activity(resource_id,id);
CREATE TRIGGER IF NOT EXISTS reminder_inventory_deleted AFTER DELETE ON operational_inventory
BEGIN
    DELETE FROM reminder_reviews WHERE resource_id=old.id;
    DELETE FROM reminder_activity WHERE resource_id=old.id;
END;
