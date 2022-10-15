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
	VALUE IS NULL OR (
		(VALUE).guild_id IS NOT NULL
		AND (VALUE).guild_id > 0
		AND (VALUE).channel_id IS NOT NULL
		AND (VALUE).channel_id > 0
	)
);

CREATE TYPE member_t AS (
	guild_id int8,
	user_id int8
);

CREATE DOMAIN member AS member_t CHECK (
	VALUE IS NULL OR (
		(VALUE).guild_id IS NOT NULL
		AND (VALUE).guild_id > 0
		AND (VALUE).user_id IS NOT NULL
		AND (VALUE).user_id > 0
	)
);

CREATE TYPE message_t AS (channel channel, message_id int8);

CREATE DOMAIN "message" AS message_t CHECK (
	VALUE IS NULL OR (
		(VALUE).channel IS NOT NULL
		AND (VALUE).message_id IS NOT NULL
		AND (VALUE).message_id > 0
	)
);
