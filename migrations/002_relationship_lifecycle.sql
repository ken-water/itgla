ALTER TABLE relationships ADD COLUMN archived_at TEXT;

INSERT OR IGNORE INTO relationships (source_asset_id, target_asset_id, kind)
SELECT source.id, target.id, 'uses_domain'
FROM assets source, assets target
WHERE source.name = 'app.cloudnote.io' AND target.name = 'cloudnote.io';

INSERT OR IGNORE INTO relationships (source_asset_id, target_asset_id, kind)
SELECT source.id, target.id, 'protected_by'
FROM assets source, assets target
WHERE source.name = 'app.cloudnote.io' AND target.name = '*.cloudnote.io';

INSERT OR IGNORE INTO relationships (source_asset_id, target_asset_id, kind)
SELECT source.id, target.id, 'deploys_to'
FROM assets source, assets target
WHERE source.name = 'notes-api' AND target.name = 'cn-prod-01';

INSERT OR IGNORE INTO relationships (source_asset_id, target_asset_id, kind)
SELECT source.id, target.id, 'depends_on'
FROM assets source, assets target
WHERE source.name = 'sync-worker' AND target.name = 'notes-api';

INSERT OR IGNORE INTO relationships (source_asset_id, target_asset_id, kind)
SELECT source.id, target.id, 'uses_domain'
FROM assets source, assets target
WHERE source.project_id = 2 AND target.project_id = 2
  AND source.kind = 'website' AND target.kind = 'domain';

INSERT OR IGNORE INTO relationships (source_asset_id, target_asset_id, kind)
SELECT source.id, target.id, 'deploys_to'
FROM assets source, assets target
WHERE source.name = 'checkout-api' AND target.name = 'sf-prod-eu';

INSERT OR IGNORE INTO relationships (source_asset_id, target_asset_id, kind)
SELECT source.id, target.id, 'protected_by'
FROM assets source, assets target
WHERE source.name = 'gateway' AND target.name = 'api.northstar.dev';

INSERT OR IGNORE INTO relationships (source_asset_id, target_asset_id, kind)
SELECT source.id, target.id, 'deploys_to'
FROM assets source, assets target
WHERE source.name = 'gateway' AND target.name = 'ns-edge-01';
