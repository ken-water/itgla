CREATE TABLE IF NOT EXISTS discovery_warnings (
    run_id INTEGER NOT NULL REFERENCES discovery_runs(id) ON DELETE CASCADE,
    message TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS discovery_reviews (
    id INTEGER PRIMARY KEY,
    observation_id INTEGER NOT NULL REFERENCES discovery_observations(id) ON DELETE CASCADE,
    previous_status TEXT NOT NULL,
    status TEXT NOT NULL,
    reviewed_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
