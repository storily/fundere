-- CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
-- CREATE FUNCTION gen_random_uuid()
-- RETURNS uuid
-- LANGUAGE SQL STABLE PARALLEL SAFE
-- AS 'select uuid_generate_v4()';

CREATE TABLE migrations (
	n serial primary key,
	name text unique,
	applied_on timestamp with time zone not null default current_timestamp
);

CREATE TYPE channel AS (
	guild_id int8,
	channel_id int8
);

CREATE TYPE member AS (
	guild_id int8,
	user_id int8
);

CREATE TYPE "message" AS (channel channel, message_id int8);
