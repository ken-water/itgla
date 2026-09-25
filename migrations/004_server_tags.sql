ALTER TABLE server_records ADD COLUMN tags TEXT NOT NULL DEFAULT '';

UPDATE server_records
SET tags = purpose
WHERE length(trim(tags)) = 0;
