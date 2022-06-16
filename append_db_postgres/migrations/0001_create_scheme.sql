create table updates(
    id serial primary key,
    created timestamp not null,
    version smallint not null,
    tag text not null,
    body jsonb not null
);