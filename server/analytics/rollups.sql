-- Keep additive counts and anonymous visitor keys, so period UV stays exact.
create table if not exists analytics_daily_facts (
    day date not null,
    event_name text not null,
    path text not null,
    visitor_key text not null,
    country text not null,
    event_count bigint not null,
    occurred_at timestamptz not null,
    primary key(day,event_name,path,visitor_key,country)
);

create table if not exists analytics_dirty_days (day date primary key);
create table if not exists analytics_rollup_state (
    id integer primary key check(id=1),
    initialized_at timestamptz not null default now()
);

create or replace function itgla_mark_dirty_day() returns trigger language plpgsql as $$
begin
    if TG_OP <> 'INSERT' then
        insert into analytics_dirty_days values ((old.occurred_at at time zone 'UTC')::date) on conflict do nothing;
    end if;
    if TG_OP <> 'DELETE' then
        insert into analytics_dirty_days values ((new.occurred_at at time zone 'UTC')::date) on conflict do nothing;
    end if;
    return null;
end;
$$;
drop trigger if exists itgla_events_dirty_day on analytics_events;
create trigger itgla_events_dirty_day after insert or update or delete on analytics_events
for each row execute function itgla_mark_dirty_day();

-- One-time historical bootstrap; ordinary restarts keep existing daily facts.
insert into analytics_dirty_days
select distinct (occurred_at at time zone 'UTC')::date from analytics_events
where not exists(select 1 from analytics_rollup_state where id=1)
on conflict do nothing;
insert into analytics_rollup_state(id) values(1) on conflict do nothing;

-- Retain rolling-window semantics. Only the first partial day and today read
-- raw events; completed middle days use their saved daily aggregates.
create or replace function itgla_report_events(window_days integer)
returns table(event_name text,path text,visitor_key text,country_name text,country_code text,
              visitor_quality text,occurred_at timestamptz,event_count bigint)
language sql stable as $$
    select f.event_name,f.path,f.visitor_key,f.country,null::text,'meaningful'::text,f.occurred_at,f.event_count
    from analytics_daily_facts f
    where f.day > ((now() - window_days * interval '1 day') at time zone 'UTC')::date
      and f.day < (now() at time zone 'UTC')::date
    union all
    select e.event_name,e.path,e.visitor_key,e.country_name,e.country_code,e.visitor_quality,e.occurred_at,1::bigint
    from analytics_events e
    where e.visitor_quality='meaningful'
      and e.occurred_at >= now() - window_days * interval '1 day'
      and (e.occurred_at < ((((now() - window_days * interval '1 day') at time zone 'UTC')::date + 1)::timestamp at time zone 'UTC')
           or e.occurred_at >= ((now() at time zone 'UTC')::date::timestamp at time zone 'UTC'));
$$;
