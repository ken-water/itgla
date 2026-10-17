CREATE TABLE work_items (
    id INTEGER PRIMARY KEY,
    title TEXT NOT NULL CHECK (length(trim(title)) BETWEEN 1 AND 200),
    kind TEXT NOT NULL CHECK (kind IN ('change', 'task', 'renewal', 'incident', 'migration', 'launch', 'maintenance')),
    status TEXT NOT NULL DEFAULT 'inbox' CHECK (status IN ('inbox', 'planned', 'ready', 'in_progress', 'blocked', 'done', 'cancelled')),
    priority TEXT NOT NULL DEFAULT 'P2' CHECK (length(trim(priority)) BETWEEN 1 AND 48),
    owner TEXT NOT NULL DEFAULT '' CHECK (length(owner) <= 160),
    start_at TEXT NOT NULL DEFAULT '' CHECK (length(start_at) <= 80),
    due_at TEXT NOT NULL DEFAULT '' CHECK (length(due_at) <= 80),
    notes TEXT NOT NULL DEFAULT '' CHECK (length(notes) <= 4000),
    position INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    archived_at TEXT
);

CREATE TABLE checklist_items (
    id INTEGER PRIMARY KEY,
    work_item_id INTEGER NOT NULL REFERENCES work_items(id) ON DELETE CASCADE,
    title TEXT NOT NULL CHECK (length(trim(title)) BETWEEN 1 AND 240),
    completed INTEGER NOT NULL DEFAULT 0 CHECK (completed IN (0, 1)),
    position INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX work_items_active_idx ON work_items(archived_at, status, position, id);
CREATE INDEX checklist_items_work_idx ON checklist_items(work_item_id, position, id);

INSERT INTO work_items (title, kind, status, priority, due_at, notes, position)
SELECT name, 'launch', 'planned', 'P2', publish_at,
       CASE WHEN trim(channel) = '' THEN '' ELSE 'Channel: ' || channel END ||
       CASE WHEN trim(media) = '' THEN '' ELSE CASE WHEN trim(channel) = '' THEN '' ELSE ' · ' END || 'Media: ' || media END,
       position
FROM operations
WHERE archived_at IS NULL;

INSERT INTO checklist_items (work_item_id, title, position)
SELECT id, 'Confirm the work scope', 0 FROM work_items;

PRAGMA user_version = 8;
