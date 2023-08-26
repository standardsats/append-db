create table updates(
    id serial primary key,
    created timestamp not null,
    version smallint not null,
    tag text not null,
    body jsonb not null
);

create table updates2(
    id serial primary key,
    created timestamp not null,
    version smallint not null,
    tag text not null,
    body jsonb not null
);

create table uuid_test(
    id serial primary key,
    u_id uuid not null
);