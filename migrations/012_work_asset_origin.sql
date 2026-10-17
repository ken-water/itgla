ALTER TABLE work_item_assets
ADD COLUMN origin TEXT NOT NULL DEFAULT 'direct'
CHECK (origin IN ('direct', 'manual'));
PRAGMA user_version = 12;
