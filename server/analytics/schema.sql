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
    visitor_quality text not null default 'meaningful',
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
alter table analytics_events add column if not exists visitor_quality text not null default 'meaningful';

do $$
begin
  alter table analytics_events drop constraint if exists analytics_events_visitor_quality_check;
  alter table analytics_events add constraint analytics_events_visitor_quality_check
    check (visitor_quality in ('meaningful', 'probe'));
exception when duplicate_object then null;
end $$;

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

create table if not exists feedback_submissions (
    id bigserial primary key,
    message text not null,
    reply_email text,
    email_status text not null default 'pending'
        check (email_status in ('pending', 'sent', 'failed', 'not_configured')),
    email_provider_id text,
    created_at timestamptz not null default now(),
    read_at timestamptz
);

create index if not exists idx_itgla_feedback_created_at
    on feedback_submissions (created_at desc);

create table if not exists email_signin_tokens (
    token_hash text primary key,
    email text not null,
    request_id text,
    expires_at timestamptz not null,
    used_at timestamptz,
    created_at timestamptz not null default now()
);

alter table email_signin_tokens add column if not exists request_id text;

create index if not exists idx_itgla_email_signin_expires_at
    on email_signin_tokens (expires_at);

create table if not exists email_login_attempts (
    request_id text primary key,
    email text not null,
    verified_at timestamptz,
    expires_at timestamptz not null,
    created_at timestamptz not null default now()
);

create index if not exists idx_itgla_email_login_attempts_expires_at
    on email_login_attempts (expires_at);

create table if not exists users (
    email text primary key,
    created_at timestamptz not null default now(),
    last_sign_in_at timestamptz not null default now()
);

create table if not exists user_sessions (
    token_hash text primary key,
    email text not null,
    expires_at timestamptz not null,
    created_at timestamptz not null default now()
);

create table if not exists user_servers (
    id bigserial primary key,
    email text not null references users(email) on delete cascade,
    name text not null check (length(trim(name)) between 1 and 160),
    tags text not null default '',
    ip_address text not null default '',
    ports text not null default '',
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now()
);

create index if not exists idx_itgla_user_servers_email
    on user_servers (email, updated_at desc);
