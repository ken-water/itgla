create table if not exists analytics_events (
    id bigserial primary key,
    event_key text not null unique,
    event_name text not null check (event_name in ('page_view', 'download_page_view', 'download_success', 'download_failure', 'download', 'page_error')),
    path text not null,
    method text not null,
    status_code integer not null,
    bytes_sent bigint not null default 0,
    referrer text,
    user_agent text,
    visitor_key text not null,
    ip_address inet,
    country_code text,
    country_name text,
    region text,
    city text,
    latitude double precision,
    longitude double precision,
    timezone text,
    asn text,
    organization text,
    isp text,
    geo_source text,
    occurred_at timestamptz not null,
    created_at timestamptz not null default now()
);

alter table analytics_events add column if not exists ip_address inet;
alter table analytics_events add column if not exists country_code text;
alter table analytics_events add column if not exists country_name text;
alter table analytics_events add column if not exists region text;
alter table analytics_events add column if not exists city text;
alter table analytics_events add column if not exists latitude double precision;
alter table analytics_events add column if not exists longitude double precision;
alter table analytics_events add column if not exists timezone text;
alter table analytics_events add column if not exists asn text;
alter table analytics_events add column if not exists organization text;
alter table analytics_events add column if not exists isp text;
alter table analytics_events add column if not exists geo_source text;

do $$
begin
  alter table analytics_events drop constraint if exists analytics_events_event_name_check;
  alter table analytics_events add constraint analytics_events_event_name_check
    check (event_name in ('page_view', 'download_page_view', 'download_success', 'download_failure', 'download', 'page_error'));
exception when duplicate_object then null;
end $$;

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
