-- Rename the table from nanowrimo_logins to trackbear_logins
ALTER TABLE nanowrimo_logins RENAME TO trackbear_logins;

-- Drop the old username and password columns
ALTER TABLE trackbear_logins DROP COLUMN username;
ALTER TABLE trackbear_logins DROP COLUMN password;

-- Add the new api_key column
ALTER TABLE trackbear_logins ADD COLUMN api_key text NOT NULL DEFAULT '';

-- Remove the default constraint now that existing rows have been updated
ALTER TABLE trackbear_logins ALTER COLUMN api_key DROP DEFAULT;
