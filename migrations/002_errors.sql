CREATE TABLE errors (
	id uuid primary key default gen_random_uuid(),
	created_at timestamp with time zone not null default current_timestamp,
	member member not null,
	"message" text not null,
	reported boolean not null default false
);
