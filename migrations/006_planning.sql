CREATE TABLE project_plans (
    project_id INTEGER PRIMARY KEY REFERENCES projects(id) ON DELETE CASCADE,
    start_at TEXT NOT NULL DEFAULT '',
    priority TEXT NOT NULL DEFAULT 'P2',
    launch_at TEXT NOT NULL DEFAULT '',
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE operations (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL CHECK (length(trim(name)) BETWEEN 1 AND 160),
    publish_at TEXT NOT NULL DEFAULT '',
    media TEXT NOT NULL DEFAULT '',
    channel TEXT NOT NULL DEFAULT '',
    position INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    archived_at TEXT
);

CREATE TABLE priority_options (
    id INTEGER PRIMARY KEY,
    label TEXT NOT NULL COLLATE NOCASE UNIQUE CHECK (length(trim(label)) BETWEEN 1 AND 48),
    color TEXT NOT NULL DEFAULT '#126a57',
    position INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE media_options (
    id INTEGER PRIMARY KEY,
    label TEXT NOT NULL COLLATE NOCASE UNIQUE CHECK (length(trim(label)) BETWEEN 1 AND 48),
    position INTEGER NOT NULL DEFAULT 0
);

INSERT INTO priority_options (label, color, position) VALUES
    ('P0', '#b64141', 0),
    ('P1', '#bf7b26', 1),
    ('P2', '#2563a6', 2),
    ('P3', '#126a57', 3);

INSERT INTO media_options (label, position) VALUES
    ('X', 0),
    ('TikTok', 1),
    ('Media', 2);

INSERT INTO project_plans (project_id, start_at, priority, launch_at)
SELECT id,
       CASE id WHEN 1 THEN 'Started 266 days ago'
               WHEN 2 THEN 'Started 244 days ago'
               WHEN 3 THEN 'Started 218 days ago'
               ELSE '' END,
       CASE id WHEN 1 THEN 'P1' WHEN 2 THEN 'P0' WHEN 3 THEN 'P2' ELSE 'P2' END,
       CASE id WHEN 1 THEN 'Launch in 166 days'
               WHEN 2 THEN 'Launch in 192 days'
               WHEN 3 THEN 'Launch in 239 days'
               ELSE '' END
FROM projects;

INSERT INTO operations (name, publish_at, media, channel, position) VALUES
    ('Launch campaign', 'Publish in 166 days', 'X', 'Product launch', 0),
    ('Creator update', 'Publish in 192 days', 'TikTok', 'Short video', 1),
    ('Press note', 'Publish in 208 days', 'Media', 'Announcement', 2);
