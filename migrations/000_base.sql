CREATE TABLE migrations (
	n serial primary key,
	name text unique,
	applied_on timestamp with time zone not null default current_timestamp
);

CREATE TYPE channel_t AS (
	guild_id int8,
	channel_id int8
);

CREATE DOMAIN channel AS channel_t
CHECK (
	(VALUE).guild_id IS NOT NULL AND
	(VALUE).channel_id IS NOT NULL
);

CREATE TYPE member_t AS (
	guild_id int8,
	user_id int8
);

CREATE DOMAIN member AS member_t CHECK (
	(VALUE).guild_id IS NOT NULL AND
	(VALUE).user_id IS NOT NULL
);
