import fs from 'node:fs';
import crypto from 'node:crypto';
import assert from 'node:assert/strict';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

const connectionString = process.env.ITGLA_TEST_DATABASE_URL;
test('incremental daily reports equal raw reports and publish atomic snapshots', {skip: !connectionString}, async () => {
const { default: pg } = await import('pg');
const root = fileURLToPath(new URL('.', import.meta.url));
const namespace='itgla_test_rollup_'+crypto.randomBytes(6).toString('hex');
const admin=new pg.Pool({connectionString:connectionString,max:1});
const pool=new pg.Pool({connectionString:connectionString,max:5,options:`-c search_path=${namespace},public`});
const AsyncFunction=Object.getPrototypeOf(async()=>{}).constructor;
function service(file, raw = false){
 let source=fs.readFileSync(file,'utf8').replaceAll('\r\n','\n');
 if (raw) {
  source = source.replaceAll('from itgla_report_events($1)', 'from analytics_events')
    .replace(/coalesce\(sum\(event_count\) filter \(where ([^\n]+?)\),0\)::int/g, 'count(*) filter (where $1)::int')
    .replaceAll('coalesce(sum(event_count),0)::int', 'count(*)::int');
 }
 const body=source.slice(source.indexOf('async function overview('),source.indexOf('async function handle('));
 return new AsyncFunction('pool','ingestAccessLog','console',body+'\nreturn {overview,daily,countries,downloadCountries,pages,downloadSummary'+(body.includes('refreshDailyFacts')?',refreshDailyFacts,refreshReportSnapshots,readReportSnapshot':'')+'};')(pool,async()=>{}, {info:()=>{}});
}
const stable=value=>Array.isArray(value)?value.map(stable).sort((a,b)=>JSON.stringify(a).localeCompare(JSON.stringify(b))):value&&typeof value==='object'&&!(value instanceof Date)?Object.fromEntries(Object.entries(value).sort().map(([k,v])=>[k,stable(v)])):value;
try {
 await admin.query(`create schema ${namespace}`);
 await pool.query(`create table analytics_events (id bigint,event_name text,path text,visitor_key text,country_name text,country_code text,visitor_quality text,occurred_at timestamptz,status_code integer,bytes_sent bigint,referrer text,user_agent text,method text)`);
 await pool.query(`insert into analytics_events select n,'download_success','/downloads/test','visitor-'||(n%3),'Test','TT','meaningful',now()-n*interval '1 day' from generate_series(1,40) n`);
 await pool.query(`insert into analytics_events select 100+n,'download_page_view','/downloads.html','visitor-'||(n%3),'Test','TT','meaningful',now()-n*interval '1 day' from generate_series(1,40) n`);
 await pool.query(`insert into analytics_events(id,event_name,path,visitor_key,country_name,country_code,visitor_quality,occurred_at) values(201,'download_success','/downloads/test','visitor-1','Test','TT','meaningful',now()),(202,'download_success','/downloads/test','visitor-1','Test','TT','meaningful',now())`);
 const schema=fs.readFileSync(root+'schema.sql','utf8');
 await pool.query(schema.slice(schema.indexOf('create table if not exists analytics_report_snapshots'),schema.indexOf('create table if not exists analytics_log_offsets')));
 await pool.query(fs.readFileSync(root+'rollups.sql','utf8'));
 const baseline=await service(root+'server.mjs',true);
 const candidate=await service(root+'server.mjs');
 const bootstrap=await candidate.refreshDailyFacts();
 for(const days of [7,30,90,365])for(const name of ['overview','daily','countries','downloadCountries','pages','downloadSummary']) {
   // Freeze now() across baseline and candidate using one repeatable-read client.
   const client=await pool.connect();
   await client.query('begin isolation level repeatable read');
   const original=pool.query.bind(pool);pool.query=client.query.bind(client);
   try{assert.deepEqual(stable(await candidate[name](days)),stable(await baseline[name](days)),`${name} ${days}`);}
   finally{pool.query=original;await client.query('rollback');client.release();}
 }
 const second=await candidate.refreshDailyFacts();
 assert.deepEqual(second,[new Date().toISOString().slice(0,10)]);
 const historical=(await pool.query("select ((now() at time zone 'UTC')::date-3)::text as day")).rows[0].day;
 await pool.query("update analytics_events set country_name='Test late correction' where id=(select id from analytics_events where (occurred_at at time zone 'UTC')::date=$1::date limit 1)",[historical]);
 const repaired=await candidate.refreshDailyFacts();assert.ok(repaired.includes(historical));
 const refreshed=await candidate.refreshReportSnapshots();
 const rows=(await pool.query('select count(*)::int as count,count(distinct computed_at)::int as versions from analytics_report_snapshots')).rows[0];
 assert.equal(rows.count,32);assert.equal(rows.versions,1);
 const snapshot=await candidate.readReportSnapshot('overview',30);
 assert.ok(snapshot.payload.latest_event_at);assert.ok(refreshed);
 await pool.query(`create function fail_snapshot_publication() returns trigger language plpgsql as $$begin raise exception 'simulated publication failure'; end;$$`);
 await pool.query('create trigger fail_snapshot_publication before update on analytics_report_snapshots for each row execute function fail_snapshot_publication()');
 await assert.rejects(candidate.refreshReportSnapshots(), /simulated publication failure/);
 const preserved=await candidate.readReportSnapshot('overview',30);
 assert.deepEqual(preserved,snapshot);
 await pool.query('drop trigger fail_snapshot_publication on analytics_report_snapshots');
 const [first,joined]=await Promise.all([candidate.refreshReportSnapshots(),candidate.refreshReportSnapshots()]);
 assert.deepEqual(first,joined);
 console.log(JSON.stringify({pass:true,bootstrapDays:bootstrap.length,secondPassDays:second,lateCorrectionDays:repaired,comparedReports:24,snapshots:rows.count,atomicVersions:rows.versions,failedPublicationPreservesPrevious:true,concurrentRefreshCoalesced:true}));
} finally {
 await pool.end();
 assert.match(namespace,/^itgla_test_rollup_[0-9a-f]{12}$/);
 await admin.query(`drop schema if exists ${namespace} cascade`);
 await admin.end();
}

});
