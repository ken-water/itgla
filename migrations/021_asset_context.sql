CREATE TABLE IF NOT EXISTS asset_context (
    resource_type TEXT NOT NULL CHECK(resource_type IN ('business','server','service','domain')),
    resource_id INTEGER NOT NULL,
    data TEXT NOT NULL DEFAULT '{}' CHECK(json_valid(data)),
    verified_at TEXT,
    verified_by TEXT NOT NULL DEFAULT '',
    PRIMARY KEY(resource_type,resource_id)
);
INSERT OR IGNORE INTO asset_context(resource_type,resource_id,verified_at)
SELECT CASE kind WHEN 0 THEN 'service' ELSE 'domain' END,id,verified_at FROM operational_inventory;

CREATE TRIGGER IF NOT EXISTS context_server_updated AFTER UPDATE OF name,purpose,tags,ip_address,ports,environment,status ON server_records
WHEN old.name IS NOT new.name OR old.purpose IS NOT new.purpose OR old.tags IS NOT new.tags OR old.ip_address IS NOT new.ip_address OR old.ports IS NOT new.ports OR old.environment IS NOT new.environment OR old.status IS NOT new.status
BEGIN UPDATE asset_context SET verified_at=NULL,verified_by='' WHERE resource_type='server' AND resource_id=new.id; END;
CREATE TRIGGER IF NOT EXISTS context_business_updated AFTER UPDATE OF name,tags,notes ON businesses
WHEN old.name IS NOT new.name OR old.tags IS NOT new.tags OR old.notes IS NOT new.notes
BEGIN UPDATE asset_context SET verified_at=NULL,verified_by='' WHERE resource_type='business' AND resource_id=new.id; END;
CREATE TRIGGER IF NOT EXISTS context_inventory_updated AFTER UPDATE OF data ON operational_inventory WHEN old.data IS NOT new.data
BEGIN UPDATE asset_context SET verified_at=NULL,verified_by='' WHERE resource_type=CASE new.kind WHEN 0 THEN 'service' ELSE 'domain' END AND resource_id=new.id; UPDATE operational_inventory SET verified_at=NULL WHERE id=new.id; END;
CREATE TRIGGER IF NOT EXISTS context_host_added AFTER INSERT ON operational_hosts
BEGIN UPDATE asset_context SET verified_at=NULL,verified_by='' WHERE resource_type IN ('service','domain') AND resource_id=new.inventory_id; UPDATE operational_inventory SET verified_at=NULL WHERE id=new.inventory_id; END;
CREATE TRIGGER IF NOT EXISTS context_host_removed AFTER DELETE ON operational_hosts
BEGIN UPDATE asset_context SET verified_at=NULL,verified_by='' WHERE resource_type IN ('service','domain') AND resource_id=old.inventory_id; UPDATE operational_inventory SET verified_at=NULL WHERE id=old.inventory_id; END;
CREATE TRIGGER IF NOT EXISTS context_server_deleted AFTER DELETE ON server_records
BEGIN DELETE FROM asset_context WHERE resource_type='server' AND resource_id=old.id; END;
CREATE TRIGGER IF NOT EXISTS context_business_deleted AFTER DELETE ON businesses
BEGIN DELETE FROM asset_context WHERE resource_type='business' AND resource_id=old.id; END;
CREATE TRIGGER IF NOT EXISTS context_inventory_deleted AFTER DELETE ON operational_inventory
BEGIN DELETE FROM asset_context WHERE resource_type=CASE old.kind WHEN 0 THEN 'service' ELSE 'domain' END AND resource_id=old.id; END;
