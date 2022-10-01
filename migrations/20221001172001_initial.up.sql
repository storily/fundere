CREATE TYPE channel_t AS (
	guild_id int,
	channel_id int
);

CREATE DOMAIN channel AS channel_t
CHECK (
	(VALUE).guild_id IS NOT NULL AND
	(VALUE).channel_id IS NOT NULL
);

CREATE TYPE member_t AS (
	guild_id int,
	user_id int
);

CREATE DOMAIN member AS member_t CHECK (
	(VALUE).guild_id IS NOT NULL AND
	(VALUE).user_id IS NOT NULL
);

CREATE TABLE sprints (
	id uuid primary key default gen_random_uuid(),
	shortid serial,
	created_at timestamp with time zone not null default current_timestamp,
	updated_at timestamp with time zone not null default current_timestamp,
	cancelled_at timestamp with time zone null,
	starting_at timestamp with time zone not null,
	duration_minutes int not null default 15,
	channels channel[] not null default '{}',

	unique (shortid)
);

CREATE INDEX sprints_starting_at ON sprints (starting_at);

CREATE TABLE sprint_participant (
	sprint_id uuid not null references sprints (id) on delete cascade,
	member member not null,
	words_start int null,
	words_end int null,

	primary key (sprint_id, member)
);
