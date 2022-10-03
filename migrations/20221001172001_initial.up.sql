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

CREATE TABLE sprints (
	id uuid primary key default gen_random_uuid(),
	shortid serial,

	created_at timestamp with time zone not null default current_timestamp,
	updated_at timestamp with time zone not null default current_timestamp,
	cancelled_at timestamp with time zone null,

	starting_at timestamp with time zone not null,
	duration interval not null,

	status text not null default 'initial',
	channels channel[] not null default '{}',

	unique (shortid)
);

CREATE INDEX sprints_starting_at ON sprints (starting_at);
CREATE INDEX sprints_cancelled ON sprints ((cancelled_at is null));
CREATE INDEX sprints_current_idx ON sprints ((cancelled_at is not null), starting_at);

CREATE TABLE sprint_participants (
	sprint_id uuid not null references sprints (id) on delete cascade,
	member member not null,
	joined_at timestamp with time zone not null default current_timestamp,

	words_start int null,
	words_end int null,

	primary key (sprint_id, member)
);

CREATE VIEW sprints_current AS
	SELECT sprints.*, array_agg(sprint_participants.*) AS participants
	FROM sprints
	LEFT JOIN sprint_participants ON sprints.id = sprint_participants.sprint_id
	WHERE true
		AND sprints.cancelled_at IS NULL
		AND (
			sprints.starting_at >= current_timestamp
			OR sprints.starting_at + sprints.duration >= current_timestamp
		)
	GROUP BY sprints.id;