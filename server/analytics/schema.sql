create table if not exists analytics_events (
    id bigserial primary key,
    event_key text not null unique,
    event_name text not null check (event_name in ('page_view', 'download', 'page_error')),
    path text not null,
    method text not null,
    status_code integer not null,
    bytes_sent bigint not null default 0,
    referrer text,
    user_agent text,
    visitor_key text not null,
    occurred_at timestamptz not null,
    created_at timestamptz not null default now()
);

create index if not exists idx_itgla_events_occurred_at
    on analytics_events (occurred_at desc);

create index if not exists idx_itgla_events_name_time
    on analytics_events (event_name, occurred_at desc);

create index if not exists idx_itgla_events_path_time
    on analytics_events (path, occurred_at desc);

create table if not exists analytics_log_offsets (
    source_path text primary key,
    byte_offset bigint not null default 0,
    updated_at timestamptz not null default now()
);

create table if not exists admin_sessions (
    token_hash text primary key,
    username text not null,
    expires_at timestamptz not null,
    created_at timestamptz not null default now()
);

create index if not exists idx_itgla_sessions_expires_at
    on admin_sessions (expires_at);
