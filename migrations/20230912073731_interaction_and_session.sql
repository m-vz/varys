create table session (
    id serial primary key,
    version text not null,
    started timestamptz not null,
    ended timestamptz
);

create table interaction (
    id serial primary key,
    started timestamptz not null,
    ended timestamptz,
    session_id int not null,
    constraint fk_session foreign key (session_id) references session(id)
);
