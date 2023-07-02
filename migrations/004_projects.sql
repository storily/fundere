CREATE TABLE projects (
	id uuid primary key default gen_random_uuid(),
	created_at timestamp with time zone not null default current_timestamp,
	updated_at timestamp with time zone not null default current_timestamp,

	member member not null,
	nano_id int8 not null,
	goal integer null,

	CHECK (goal >= 0),
	UNIQUE (member)
);
