ALTER TABLE server_records ADD COLUMN name TEXT NOT NULL DEFAULT '';
ALTER TABLE server_records ADD COLUMN environment TEXT NOT NULL DEFAULT 'production';
ALTER TABLE server_records ADD COLUMN status TEXT NOT NULL DEFAULT 'active';

UPDATE server_records
SET name = COALESCE(
        NULLIF(trim((SELECT value FROM server_custom_values v
                     JOIN server_custom_columns c ON c.id = v.column_id
                     WHERE v.server_id = server_records.id AND lower(c.name) = 'name')), ''),
        NULLIF(trim(purpose), ''),
        ip_address
    ),
    environment = COALESCE(
        NULLIF(trim((SELECT value FROM server_custom_values v
                     JOIN server_custom_columns c ON c.id = v.column_id
                     WHERE v.server_id = server_records.id AND lower(c.name) = 'environment')), ''),
        'production'
    ),
    status = COALESCE(
        NULLIF(trim((SELECT value FROM server_custom_values v
                     JOIN server_custom_columns c ON c.id = v.column_id
                     WHERE v.server_id = server_records.id AND lower(c.name) = 'status')), ''),
        'active'
    ),
    purpose = COALESCE(
        NULLIF(trim((SELECT value FROM server_custom_values v
                     JOIN server_custom_columns c ON c.id = v.column_id
                     WHERE v.server_id = server_records.id AND lower(c.name) = 'purpose')), ''),
        purpose
    );

DELETE FROM server_custom_values
WHERE column_id IN (
    SELECT id FROM server_custom_columns
    WHERE lower(name) IN ('name', 'environment', 'status', 'purpose')
);

DELETE FROM server_custom_columns
WHERE lower(name) IN ('name', 'environment', 'status', 'purpose');
