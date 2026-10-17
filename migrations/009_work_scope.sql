CREATE TABLE work_item_projects (
    work_item_id INTEGER NOT NULL REFERENCES work_items(id) ON DELETE CASCADE,
    project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    PRIMARY KEY (work_item_id, project_id)
);
CREATE TABLE work_item_servers (
    work_item_id INTEGER NOT NULL REFERENCES work_items(id) ON DELETE CASCADE,
    server_id INTEGER NOT NULL REFERENCES server_records(id) ON DELETE CASCADE,
    PRIMARY KEY (work_item_id, server_id)
);
CREATE INDEX work_item_projects_project_idx ON work_item_projects(project_id, work_item_id);
CREATE INDEX work_item_servers_server_idx ON work_item_servers(server_id, work_item_id);
PRAGMA user_version = 9;
