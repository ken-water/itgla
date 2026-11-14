CREATE TABLE IF NOT EXISTS discovery_asset_links (
    observation_id INTEGER PRIMARY KEY REFERENCES discovery_observations(id) ON DELETE CASCADE,
    asset_kind TEXT NOT NULL CHECK(asset_kind IN ('domain','site')),
    asset_id INTEGER NOT NULL CHECK(asset_id > 0),
    linked_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS discovery_observations_run_page_idx ON discovery_observations(run_id, id DESC);
CREATE INDEX IF NOT EXISTS discovery_reviews_observation_idx ON discovery_reviews(observation_id, id DESC);
