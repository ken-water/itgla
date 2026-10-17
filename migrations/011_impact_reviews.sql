CREATE TABLE work_item_impact_reviews (
    work_item_id INTEGER NOT NULL REFERENCES work_items(id) ON DELETE CASCADE,
    asset_id INTEGER NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
    decision TEXT NOT NULL CHECK (decision IN ('excluded')),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (work_item_id, asset_id)
);
CREATE INDEX work_item_impact_reviews_asset_idx ON work_item_impact_reviews(asset_id, work_item_id);
PRAGMA user_version = 11;
