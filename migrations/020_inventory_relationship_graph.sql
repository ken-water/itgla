CREATE TABLE IF NOT EXISTS businesses (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL CHECK(length(trim(name)) BETWEEN 1 AND 160),
    notes TEXT NOT NULL DEFAULT '' CHECK(length(notes) <= 2048),
    tags TEXT NOT NULL DEFAULT '' CHECK(length(tags) <= 2048),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    archived_at TEXT
);
CREATE UNIQUE INDEX IF NOT EXISTS businesses_name_unique ON businesses(lower(trim(name)));

ALTER TABLE operational_inventory ADD COLUMN created_at TEXT NOT NULL DEFAULT '';
ALTER TABLE operational_inventory ADD COLUMN updated_at TEXT NOT NULL DEFAULT '';
ALTER TABLE operational_inventory ADD COLUMN imported_at TEXT;
ALTER TABLE operational_inventory ADD COLUMN last_hidden_at TEXT;
ALTER TABLE operational_inventory ADD COLUMN last_restored_at TEXT;
ALTER TABLE operational_inventory ADD COLUMN business_id INTEGER REFERENCES businesses(id) ON DELETE SET NULL;
ALTER TABLE operational_inventory ADD COLUMN parent_domain_id INTEGER REFERENCES operational_inventory(id) ON DELETE SET NULL;
-- Existing inventory timestamps are unknown; leave them blank rather than inventing them.

UPDATE operational_inventory AS child
SET parent_domain_id = (
    SELECT parent.id FROM operational_inventory AS parent
    WHERE parent.kind = 1
      AND lower(json_extract(parent.data, '$.name')) = lower(json_extract(child.data, '$.parent'))
      AND parent.id <> child.id
    ORDER BY parent.id LIMIT 1
)
WHERE child.kind = 1 AND trim(json_extract(child.data, '$.parent')) <> '';

CREATE TABLE IF NOT EXISTS resource_links (
    id INTEGER PRIMARY KEY,
    source_type TEXT NOT NULL CHECK(source_type IN ('business','domain','service','server')),
    source_id INTEGER NOT NULL,
    target_type TEXT NOT NULL CHECK(target_type IN ('business','domain','service','server')),
    target_id INTEGER NOT NULL,
    relation_type TEXT NOT NULL CHECK(relation_type IN (
        'owns','contains','resolves_to','hosted_on','origin_on','depends_on',
        'replicates_to','fails_over_to','balances_to','related_to'
    )),
    notes TEXT NOT NULL DEFAULT '' CHECK(length(notes) <= 2048),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CHECK(source_type <> target_type OR source_id <> target_id),
    UNIQUE(source_type,source_id,target_type,target_id,relation_type)
);
CREATE INDEX IF NOT EXISTS resource_links_source_idx ON resource_links(source_type,source_id);
CREATE INDEX IF NOT EXISTS resource_links_target_idx ON resource_links(target_type,target_id);

INSERT INTO resource_links(source_type,source_id,target_type,target_id,relation_type,notes,created_at,updated_at)
SELECT 'service',source_id,'service',target_id,
       CASE kind WHEN 0 THEN 'depends_on' WHEN 1 THEN 'replicates_to'
                 WHEN 2 THEN 'fails_over_to' ELSE 'balances_to' END,
       notes,CURRENT_TIMESTAMP,CURRENT_TIMESTAMP
FROM operational_relations;
INSERT OR IGNORE INTO resource_links(source_type,source_id,target_type,target_id,relation_type)
SELECT CASE i.kind WHEN 0 THEN 'service' ELSE 'domain' END,
       h.inventory_id,'server',h.server_id,
       CASE i.kind WHEN 0 THEN 'hosted_on' ELSE 'origin_on' END
FROM operational_hosts h JOIN operational_inventory i ON i.id=h.inventory_id;
INSERT OR IGNORE INTO resource_links(source_type,source_id,target_type,target_id,relation_type)
SELECT 'domain',parent_domain_id,'domain',id,'contains'
FROM operational_inventory WHERE kind=1 AND parent_domain_id IS NOT NULL;
INSERT OR IGNORE INTO resource_links(source_type,source_id,target_type,target_id,relation_type)
SELECT 'business',business_id,'domain',id,'owns'
FROM operational_inventory WHERE kind=1 AND business_id IS NOT NULL;

CREATE TABLE IF NOT EXISTS resource_history (
    id INTEGER PRIMARY KEY,
    resource_type TEXT NOT NULL CHECK(resource_type IN ('business','domain','service','server','link')),
    resource_id INTEGER NOT NULL,
    action TEXT NOT NULL CHECK(action IN ('created','updated','hidden','restored','verified','credential_updated','linked','unlinked','baseline')),
    summary TEXT NOT NULL CHECK(length(summary) <= 512),
    occurred_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS resource_history_lookup ON resource_history(resource_type,resource_id,occurred_at DESC,id DESC);

-- Old server timestamps are known facts. Detailed operation history starts here.
INSERT INTO resource_history(resource_type,resource_id,action,summary,occurred_at)
SELECT 'server',id,'baseline','History tracking started for an existing server',CURRENT_TIMESTAMP
FROM server_records;
INSERT INTO resource_history(resource_type,resource_id,action,summary,occurred_at)
SELECT 'domain',id,'baseline','History tracking started for an existing domain',CURRENT_TIMESTAMP
FROM operational_inventory WHERE kind=1;
INSERT INTO resource_history(resource_type,resource_id,action,summary,occurred_at)
SELECT 'service',id,'baseline','History tracking started for an existing service',CURRENT_TIMESTAMP
FROM operational_inventory WHERE kind=0;
INSERT INTO resource_history(resource_type,resource_id,action,summary,occurred_at)
SELECT 'server',id,'hidden','Server was hidden before detailed history tracking began',archived_at
FROM server_records WHERE archived_at IS NOT NULL;

CREATE TRIGGER IF NOT EXISTS server_history_created AFTER INSERT ON server_records BEGIN
    INSERT INTO resource_history(resource_type,resource_id,action,summary)
    VALUES('server',NEW.id,'created','Server added');
END;
CREATE TRIGGER IF NOT EXISTS server_history_updated AFTER UPDATE ON server_records
WHEN OLD.name IS NOT NEW.name OR OLD.purpose IS NOT NEW.purpose OR OLD.tags IS NOT NEW.tags
  OR OLD.ip_address IS NOT NEW.ip_address OR OLD.ports IS NOT NEW.ports
  OR OLD.environment IS NOT NEW.environment OR OLD.status IS NOT NEW.status BEGIN
    INSERT INTO resource_history(resource_type,resource_id,action,summary)
    VALUES('server',NEW.id,'updated','Server details updated');
END;
CREATE TRIGGER IF NOT EXISTS server_history_hidden AFTER UPDATE OF archived_at ON server_records
WHEN OLD.archived_at IS NOT NEW.archived_at AND NEW.archived_at IS NOT NULL BEGIN
    INSERT INTO resource_history(resource_type,resource_id,action,summary,occurred_at)
    VALUES('server',NEW.id,'hidden','Server hidden',NEW.archived_at);
END;
CREATE TRIGGER IF NOT EXISTS server_history_restored AFTER UPDATE OF archived_at ON server_records
WHEN OLD.archived_at IS NOT NEW.archived_at AND NEW.archived_at IS NULL BEGIN
    INSERT INTO resource_history(resource_type,resource_id,action,summary)
    VALUES('server',NEW.id,'restored','Server restored');
END;

CREATE TRIGGER IF NOT EXISTS inventory_history_created AFTER INSERT ON operational_inventory BEGIN
    UPDATE operational_inventory SET created_at=CURRENT_TIMESTAMP,updated_at=CURRENT_TIMESTAMP WHERE id=NEW.id;
    INSERT INTO resource_history(resource_type,resource_id,action,summary)
    VALUES(CASE NEW.kind WHEN 0 THEN 'service' ELSE 'domain' END,NEW.id,'created',
           CASE NEW.kind WHEN 0 THEN 'Service added' ELSE 'Domain added' END);
END;
CREATE TRIGGER IF NOT EXISTS inventory_history_updated AFTER UPDATE ON operational_inventory
WHEN OLD.data IS NOT NEW.data OR OLD.business_id IS NOT NEW.business_id OR OLD.parent_domain_id IS NOT NEW.parent_domain_id BEGIN
    UPDATE operational_inventory SET updated_at=CURRENT_TIMESTAMP WHERE id=NEW.id;
    INSERT INTO resource_history(resource_type,resource_id,action,summary)
    VALUES(CASE NEW.kind WHEN 0 THEN 'service' ELSE 'domain' END,NEW.id,'updated',
           CASE NEW.kind WHEN 0 THEN 'Service details updated' ELSE 'Domain details updated' END);
END;
CREATE TRIGGER IF NOT EXISTS inventory_history_credentials AFTER UPDATE OF credentials ON operational_inventory
WHEN OLD.credentials IS NOT NEW.credentials BEGIN
    INSERT INTO resource_history(resource_type,resource_id,action,summary)
    VALUES('service',NEW.id,'credential_updated','Access account changed; secret values are not recorded');
END;
CREATE TRIGGER IF NOT EXISTS inventory_history_hidden AFTER UPDATE OF hidden ON operational_inventory
WHEN OLD.hidden <> NEW.hidden AND NEW.hidden=1 BEGIN
    UPDATE operational_inventory SET last_hidden_at=CURRENT_TIMESTAMP WHERE id=NEW.id;
    INSERT INTO resource_history(resource_type,resource_id,action,summary)
    VALUES(CASE NEW.kind WHEN 0 THEN 'service' ELSE 'domain' END,NEW.id,'hidden','Record hidden');
END;
CREATE TRIGGER IF NOT EXISTS inventory_history_restored AFTER UPDATE OF hidden ON operational_inventory
WHEN OLD.hidden <> NEW.hidden AND NEW.hidden=0 BEGIN
    UPDATE operational_inventory SET last_restored_at=CURRENT_TIMESTAMP WHERE id=NEW.id;
    INSERT INTO resource_history(resource_type,resource_id,action,summary)
    VALUES(CASE NEW.kind WHEN 0 THEN 'service' ELSE 'domain' END,NEW.id,'restored','Record restored');
END;
CREATE TRIGGER IF NOT EXISTS business_history_created AFTER INSERT ON businesses BEGIN
    INSERT INTO resource_history(resource_type,resource_id,action,summary)
    VALUES('business',NEW.id,'created','Business added');
END;
CREATE TRIGGER IF NOT EXISTS business_history_updated AFTER UPDATE ON businesses
WHEN OLD.name IS NOT NEW.name OR OLD.notes IS NOT NEW.notes OR OLD.tags IS NOT NEW.tags BEGIN
    INSERT INTO resource_history(resource_type,resource_id,action,summary)
    VALUES('business',NEW.id,'updated','Business details updated');
END;
CREATE TRIGGER IF NOT EXISTS business_history_hidden AFTER UPDATE OF archived_at ON businesses
WHEN OLD.archived_at IS NOT NEW.archived_at AND NEW.archived_at IS NOT NULL BEGIN
    INSERT INTO resource_history(resource_type,resource_id,action,summary,occurred_at)
    VALUES('business',NEW.id,'hidden','Business hidden',NEW.archived_at);
END;
CREATE TRIGGER IF NOT EXISTS business_history_restored AFTER UPDATE OF archived_at ON businesses
WHEN OLD.archived_at IS NOT NEW.archived_at AND NEW.archived_at IS NULL BEGIN
    INSERT INTO resource_history(resource_type,resource_id,action,summary)
    VALUES('business',NEW.id,'restored','Business restored');
END;

CREATE TRIGGER IF NOT EXISTS resource_link_history_created AFTER INSERT ON resource_links BEGIN
    INSERT INTO resource_history(resource_type,resource_id,action,summary)
    VALUES(NEW.source_type,NEW.source_id,'linked','Relationship added: ' || NEW.relation_type);
    INSERT INTO resource_history(resource_type,resource_id,action,summary)
    VALUES(NEW.target_type,NEW.target_id,'linked','Relationship added: ' || NEW.relation_type);
END;
CREATE TRIGGER IF NOT EXISTS resource_link_history_deleted AFTER DELETE ON resource_links BEGIN
    INSERT INTO resource_history(resource_type,resource_id,action,summary)
    VALUES(OLD.source_type,OLD.source_id,'unlinked','Relationship removed: ' || OLD.relation_type);
    INSERT INTO resource_history(resource_type,resource_id,action,summary)
    VALUES(OLD.target_type,OLD.target_id,'unlinked','Relationship removed: ' || OLD.relation_type);
END;
CREATE TRIGGER IF NOT EXISTS resource_link_history_updated AFTER UPDATE ON resource_links
WHEN OLD.relation_type IS NOT NEW.relation_type OR OLD.notes IS NOT NEW.notes BEGIN
    INSERT INTO resource_history(resource_type,resource_id,action,summary)
    VALUES(NEW.source_type,NEW.source_id,'linked','Relationship updated: ' || NEW.relation_type);
    INSERT INTO resource_history(resource_type,resource_id,action,summary)
    VALUES(NEW.target_type,NEW.target_id,'linked','Relationship updated: ' || NEW.relation_type);
END;

-- Keep legacy APIs and the map on one source of truth.
CREATE TRIGGER IF NOT EXISTS operational_relation_link_insert AFTER INSERT ON operational_relations BEGIN
 INSERT INTO resource_links(source_type,source_id,target_type,target_id,relation_type,notes)
 VALUES('service',NEW.source_id,'service',NEW.target_id,CASE NEW.kind WHEN 0 THEN 'depends_on' WHEN 1 THEN 'replicates_to' WHEN 2 THEN 'fails_over_to' ELSE 'balances_to' END,NEW.notes)
 ON CONFLICT(source_type,source_id,target_type,target_id,relation_type) DO UPDATE SET notes=excluded.notes;
END;
CREATE TRIGGER IF NOT EXISTS operational_relation_link_update AFTER UPDATE ON operational_relations BEGIN
 UPDATE resource_links SET notes=NEW.notes,updated_at=CURRENT_TIMESTAMP WHERE source_type='service' AND source_id=NEW.source_id AND target_type='service' AND target_id=NEW.target_id AND relation_type=CASE NEW.kind WHEN 0 THEN 'depends_on' WHEN 1 THEN 'replicates_to' WHEN 2 THEN 'fails_over_to' ELSE 'balances_to' END;
END;
CREATE TRIGGER IF NOT EXISTS operational_relation_link_delete AFTER DELETE ON operational_relations BEGIN
 DELETE FROM resource_links WHERE source_type='service' AND source_id=OLD.source_id AND target_type='service' AND target_id=OLD.target_id AND relation_type=CASE OLD.kind WHEN 0 THEN 'depends_on' WHEN 1 THEN 'replicates_to' WHEN 2 THEN 'fails_over_to' ELSE 'balances_to' END;
END;
CREATE TRIGGER IF NOT EXISTS host_link_insert AFTER INSERT ON operational_hosts BEGIN
 INSERT OR IGNORE INTO resource_links(source_type,source_id,target_type,target_id,relation_type)
 SELECT CASE kind WHEN 0 THEN 'service' ELSE 'domain' END,NEW.inventory_id,'server',NEW.server_id,CASE kind WHEN 0 THEN 'hosted_on' ELSE 'origin_on' END FROM operational_inventory WHERE id=NEW.inventory_id;
END;
CREATE TRIGGER IF NOT EXISTS host_link_delete AFTER DELETE ON operational_hosts BEGIN
 DELETE FROM resource_links WHERE source_id=OLD.inventory_id AND source_type IN ('service','domain') AND target_type='server' AND target_id=OLD.server_id AND relation_type IN ('hosted_on','origin_on');
END;
CREATE TRIGGER IF NOT EXISTS server_custom_history_insert AFTER INSERT ON server_custom_values BEGIN
 INSERT INTO resource_history(resource_type,resource_id,action,summary) VALUES('server',NEW.server_id,'updated','Server custom field added');
END;
CREATE TRIGGER IF NOT EXISTS server_custom_history_update AFTER UPDATE ON server_custom_values WHEN OLD.value IS NOT NEW.value BEGIN
 INSERT INTO resource_history(resource_type,resource_id,action,summary) VALUES('server',NEW.server_id,'updated','Server custom field updated');
END;
