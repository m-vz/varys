/*
 * This migration file will be changed until the system is running (and production data exists) at which point changes
 * will be added in new migrations.
 */

create table interactor_config (
    id serial primary key,
    interface text not null,
    voice text not null,
    sensitivity text not null,
    model text not null,
    
    unique (interface, voice, sensitivity, model)
);

create table session (
    id serial primary key,
    version text not null,
    interactor_config_id int not null,
    started timestamptz not null,
    ended timestamptz,
    
    constraint fk_interactor_config foreign key (interactor_config_id) references interactor_config(id)
);

create table interaction (
    id serial primary key,
    session_id int not null,
    query text not null,
    response text,
    started timestamptz not null,
    ended timestamptz,
    
    constraint fk_session foreign key (session_id) references session(id)
);
