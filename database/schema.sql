PRAGMA foreign_keys = ON;

create table if not exists tasks
(
    id          text primary key not null,
    title       varchar(255)     not null,
    description text             null,
    label       varchar(30)      null,
    target      varchar(30)      not null,
    dateline    datetime         not null,
    created_at  datetime         not null default (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    updated_at  datetime         not null default (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

create table if not exists reminder_jobs
(
    id         text primary key not null,
    task_id    text             not null references tasks (id),
    remind_at  datetime         not null,
    status     varchar(10)      not null default 'PENDING',
    created_at datetime         not null default (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);
