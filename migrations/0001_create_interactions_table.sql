create table interaction (
    id serial primary key,
    started timestamptz not null,
    ended timestamptz not null
);