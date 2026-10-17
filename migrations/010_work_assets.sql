CREATE TABLE work_item_assets (
    work_item_id INTEGER NOT NULL REFERENCES work_items(id) ON DELETE CASCADE,
    asset_id INTEGER NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
    PRIMARY KEY (work_item_id, asset_id)
);
CREATE INDEX work_item_assets_asset_idx ON work_item_assets(asset_id, work_item_id);
PRAGMA user_version = 10;
