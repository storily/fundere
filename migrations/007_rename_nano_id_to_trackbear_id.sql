TRUNCATE TABLE projects;
ALTER TABLE projects DROP COLUMN nano_id;
ALTER TABLE projects ADD COLUMN trackbear_id bigint not null;
