CREATE TABLE user_preferences (
	member member primary key,
	timezone text not null default 'Pacific/Auckland'
);
