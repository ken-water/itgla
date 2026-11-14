CREATE TABLE IF NOT EXISTS discovery_runs (
    id INTEGER PRIMARY KEY,
    input_url TEXT NOT NULL,
    host TEXT NOT NULL,
    started_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    completed_at TEXT,
    status TEXT NOT NULL DEFAULT 'completed',
    warning_count INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS discovery_observations (
    id INTEGER PRIMARY KEY,
    run_id INTEGER NOT NULL REFERENCES discovery_runs(id) ON DELETE CASCADE,
    kind TEXT NOT NULL,
    value TEXT NOT NULL,
    source TEXT NOT NULL,
    evidence TEXT NOT NULL DEFAULT '',
    confidence INTEGER NOT NULL CHECK(confidence BETWEEN 0 AND 100),
    verification_status TEXT NOT NULL DEFAULT 'needs-review',
    discovered_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_seen_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(run_id, kind, value, source)
);
CREATE INDEX IF NOT EXISTS discovery_observations_value_idx ON discovery_observations(value);
CREATE INDEX IF NOT EXISTS discovery_observations_status_idx ON discovery_observations(verification_status);
