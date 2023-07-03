CREATE TABLE nanowrimo_logins (
	id uuid primary key default gen_random_uuid(),
	created_at timestamp with time zone not null default current_timestamp,
	updated_at timestamp with time zone not null default current_timestamp,

	member member not null,
	username text not null,
	password text not null,

	UNIQUE (member)
);

