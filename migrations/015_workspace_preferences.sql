CREATE TABLE resource_options (
    id INTEGER PRIMARY KEY,
    kind TEXT NOT NULL CHECK (kind IN ('environment', 'status')),
    label TEXT NOT NULL COLLATE NOCASE CHECK (length(trim(label)) BETWEEN 1 AND 64),
    semantic TEXT NOT NULL DEFAULT 'neutral' CHECK (semantic IN ('healthy', 'attention', 'inactive', 'planned', 'neutral')),
    color TEXT NOT NULL DEFAULT 'slate' CHECK (color IN ('green', 'amber', 'red', 'blue', 'purple', 'cyan', 'slate')),
    position INTEGER NOT NULL DEFAULT 0,
    is_default INTEGER NOT NULL DEFAULT 0 CHECK (is_default IN (0, 1)),
    hidden_at TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(kind, label)
);

CREATE UNIQUE INDEX resource_options_one_default_idx
ON resource_options(kind) WHERE is_default = 1 AND hidden_at IS NULL;

INSERT INTO resource_options(kind,label,semantic,color,position,is_default) VALUES
('environment','Production','neutral','purple',0,1),
('environment','Staging','planned','amber',1,0),
('environment','Development','planned','blue',2,0),
('environment','Test','neutral','cyan',3,0),
('environment','Local','neutral','slate',4,0),
('status','Active','healthy','green',0,1),
('status','Maintenance','attention','amber',1,0),
('status','Attention','attention','red',2,0),
('status','Offline','inactive','slate',3,0),
('status','Planned','planned','blue',4,0),
('status','Retired','inactive','slate',5,0);

INSERT OR IGNORE INTO resource_options(kind,label,semantic,color,position,is_default)
SELECT 'environment', environment, 'neutral', 'slate',
       100 + ROW_NUMBER() OVER (ORDER BY lower(environment)), 0
FROM (
    SELECT trim(environment) AS environment FROM server_records WHERE trim(environment) <> ''
    UNION
    SELECT trim(environment) FROM sites WHERE trim(environment) <> ''
);

INSERT OR IGNORE INTO resource_options(kind,label,semantic,color,position,is_default)
SELECT 'status', status, 'neutral', 'slate',
       100 + ROW_NUMBER() OVER (ORDER BY lower(status)), 0
FROM (
    SELECT trim(status) AS status FROM server_records WHERE trim(status) <> ''
    UNION
    SELECT trim(status) FROM sites WHERE trim(status) <> ''
);

CREATE TABLE managed_tags (
    id INTEGER PRIMARY KEY,
    label TEXT NOT NULL COLLATE NOCASE UNIQUE CHECK (length(trim(label)) BETWEEN 1 AND 64),
    color TEXT NOT NULL DEFAULT 'slate' CHECK (color IN ('green', 'amber', 'red', 'blue', 'purple', 'cyan', 'slate')),
    position INTEGER NOT NULL DEFAULT 0,
    hidden_at TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO managed_tags(label,color,position) VALUES
('production','purple',0),
('customer-facing','blue',1),
('database','cyan',2),
('internal','slate',3),
('needs-attention','red',4);

ALTER TABLE server_custom_columns ADD COLUMN value_type TEXT NOT NULL DEFAULT 'text'
    CHECK (value_type IN ('text', 'number', 'date', 'url', 'boolean'));
ALTER TABLE server_custom_columns ADD COLUMN default_value TEXT NOT NULL DEFAULT ''
    CHECK (length(default_value) <= 2048);
ALTER TABLE server_custom_columns ADD COLUMN hidden_at TEXT;

INSERT OR IGNORE INTO app_settings(key,value) VALUES
('theme_index','0'),
('table_density','1'),
('show_cell_tooltips','1'),
('fit_columns_on_open','0'),
('server_default_sort','0'),
('site_default_sort','0'),
('server_visible_columns','[true,true,true,true,true,true,true]'),
('server_default_ports','22, 443'),
('server_default_tags',''),
('site_default_tags',''),
('import_duplicate_policy','0'),
('backup_retention','7');
