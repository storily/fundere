CREATE TYPE sprint_status AS ENUM (
	'Initial',
	'Announced',
	'Started',
	'Ended',
	'Summaried'
);

CREATE TABLE sprints (
	id uuid primary key default gen_random_uuid(),
	shortid serial,

	created_at timestamp with time zone not null default current_timestamp,
	updated_at timestamp with time zone not null default current_timestamp,
	cancelled_at timestamp with time zone null,

	starting_at timestamp with time zone not null,
	duration interval not null,

	status sprint_status not null default 'Initial',
	interaction_token text not null,
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
	SELECT sprints.*
	FROM sprints
	LEFT JOIN sprint_participants ON sprints.id = sprint_participants.sprint_id
	WHERE true
		AND sprints.cancelled_at IS NULL
		AND (
			sprints.starting_at >= current_timestamp
			OR sprints.starting_at + sprints.duration >= current_timestamp
		)
	GROUP BY sprints.id
	HAVING count(sprint_participants.*) > 0;

CREATE VIEW sprints_finished_but_not_summaried AS
	SELECT sprints.*
	FROM sprints
	LEFT JOIN sprint_participants ON sprints.id = sprint_participants.sprint_id
	WHERE true
		AND sprints.cancelled_at IS NULL
		AND sprints.starting_at + sprints.duration <= current_timestamp
		AND sprints.status != 'Summaried'
	GROUP BY sprints.id
	HAVING count(sprint_participants.*) > 0;
