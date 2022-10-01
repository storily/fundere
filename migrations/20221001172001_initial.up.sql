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

	starting_at timestamp with time zone not null,
	cancelled_at timestamp with time zone null,
	duration_minutes int not null default 15,
	channels channel[] not null default '{}',

	unique (shortid)
);

CREATE INDEX sprints_starting_at ON sprints (starting_at);
CREATE INDEX sprints_cancelled ON sprints ((cancelled_at is null));
CREATE INDEX sprints_current_idx ON sprints ((cancelled_at is not null), starting_at);

CREATE TABLE sprint_participant (
	sprint_id uuid not null references sprints (id) on delete cascade,
	member member not null,
	joined_at timestamp with time zone not null default current_timestamp,

	words_start int null,
	words_end int null,

	primary key (sprint_id, member)
);

CREATE VIEW sprints_current AS
	SELECT sprints.*, array_agg(sprint_participant.*) AS participants
	FROM sprints
	LEFT JOIN sprint_participant ON sprints.id = sprint_participant.sprint_id
	WHERE true
		AND sprints.starting_at >= current_timestamp
		AND sprints.starting_at + (interval '1 minute' * sprints.duration_minutes) >= current_timestamp
		AND sprints.cancelled_at IS NOT NULL
	GROUP BY sprints.id;
