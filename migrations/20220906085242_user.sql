-- Add migration script here
create table if not exists user
(
    id              integer primary key autoincrement ,
    username        varchar(50)  not null,
    nickname        varchar(50)  not null,
    email           varchar(255) not null,
    password_hash   varchar(255) not null,
    avatar          varchar(500),
    description     varchar(500),
    created_at      timestamp    not null default current_timestamp,
    updated_at      timestamp    not null default current_timestamp,
    expired_at      timestamp,
    last_login_time timestamp,
    last_login_ip   varchar(50),
    mfa_type        int          not null,
    mfa_key         varchar(255)
);