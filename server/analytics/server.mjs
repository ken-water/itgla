import crypto from "node:crypto";
import fs from "node:fs/promises";
import http from "node:http";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { Pool } from "pg";
import { adminCredentialRedirect, daysFromUrl, hash, isMeaningfulVisitor, parseLogLine, passwordDigest, publicPagePaths, safeEqual } from "./lib.mjs";
import { lookupGeo } from "./geo.mjs";

const root = path.dirname(fileURLToPath(import.meta.url));
const config = {
  port: Number(process.env.ITGLA_ANALYTICS_PORT || 3420),
  logPath: process.env.ITGLA_ACCESS_LOG || "/var/log/itgla/nginx-access.log",
  databaseUrl: process.env.ITGLA_DATABASE_URL,
  adminUsername: process.env.ITGLA_ADMIN_USERNAME,
  adminPasswordSalt: process.env.ITGLA_ADMIN_PASSWORD_SALT,
  adminPasswordScrypt: process.env.ITGLA_ADMIN_PASSWORD_SCRYPT,
  visitorSalt: process.env.ITGLA_VISITOR_SALT,
  geoIpDbPath: process.env.ITGLA_GEOIP_DB_PATH || "",
  geoProvider: process.env.ITGLA_GEOIP_PROVIDER || "local-mmdb",
  resendApiKey: process.env.RESEND_API_KEY || "",
  feedbackFrom: process.env.ITGLA_FEEDBACK_FROM || "service@itgla.com",
  publicBaseUrl: process.env.ITGLA_PUBLIC_BASE_URL || "https://itgla.com",
  geoRetentionDays: Math.max(1, Number(process.env.ITGLA_GEOIP_RETENTION_DAYS || 180)),
  retentionDays: Math.max(30, Number(process.env.ITGLA_RETENTION_DAYS || 365)),
};

for (const [name, value] of Object.entries(config).filter(([name]) => name !== "geoIpDbPath")) {
  if (value === undefined || value === "") throw new Error(`Missing required analytics configuration: ${name}`);
}

const pool = new Pool({ connectionString: config.databaseUrl, max: 5 });
const sessionTtlSeconds = 8 * 60 * 60;
const loginAttempts = new Map();

function securityHeaders(contentType) {
  return {
    "Content-Type": contentType,
    "Cache-Control": "no-store",
    "X-Content-Type-Options": "nosniff",
    "X-Frame-Options": "DENY",
    "Referrer-Policy": "no-referrer",
    "Permissions-Policy": "camera=(), microphone=(), geolocation=()",
    "Content-Security-Policy": "default-src 'none'; script-src 'self'; style-src 'self'; img-src 'self'; connect-src 'self'; base-uri 'none'; form-action 'self'; frame-ancestors 'none'",
    "X-Robots-Tag": "noindex, nofollow",
  };
}

function json(response, status, body, extraHeaders = {}) {
  if (response.adminStartedAt !== undefined) {
    response.setHeader("Server-Timing", `app;dur=${(performance.now() - response.adminStartedAt).toFixed(1)}`);
  }
  response.writeHead(status, { ...securityHeaders("application/json; charset=utf-8"), ...extraHeaders });
  response.end(JSON.stringify(body));
}

async function asset(response, fileName, contentType) {
  try {
    const body = await fs.readFile(path.join(root, "web", fileName));
    response.writeHead(200, securityHeaders(contentType));
    response.end(body);
  } catch {
    json(response, 404, { message: "Not found." });
  }
}

function readBody(request) {
  return new Promise((resolve, reject) => {
    let body = "";
    request.on("data", (chunk) => {
      body += chunk;
      if (body.length > 16_384) reject(new Error("request body too large"));
    });
    request.on("end", () => {
      try {
        resolve(body ? JSON.parse(body) : {});
      } catch {
        reject(new Error("invalid JSON"));
      }
    });
    request.on("error", reject);
  });
}

function clientAddress(request) {
  return String(request.headers["x-real-ip"] || request.socket.remoteAddress || "unknown")
    .split(",")[0]
    .trim();
}

function cookieValue(request, name) {
  const prefix = `${name}=`;
  const item = String(request.headers.cookie || "").split(";").find((value) => value.trim().startsWith(prefix));
  return item ? decodeURIComponent(item.trim().slice(prefix.length)) : null;
}

function sessionToken(request) {
  return cookieValue(request, "itgla_admin_session");
}

function userSessionToken(request) {
  return cookieValue(request, "itgla_user_session");
}

async function userEmail(request) {
  const token = userSessionToken(request);
  if (!token) return null;
  const result = await pool.query(
    "select email from user_sessions where token_hash=$1 and expires_at > now()",
    [hash(token)],
  );
  return result.rows[0]?.email || null;
}

async function authenticated(request) {
  const token = sessionToken(request);
  if (!token) return null;
  const result = await pool.query(
    "select username from admin_sessions where token_hash = $1 and expires_at > now()",
    [hash(token)],
  );
  return result.rows[0]?.username || null;
}

function sameOrigin(request) {
  const origin = String(request.headers.origin || "");
  if (!origin) return true;
  const host = String(request.headers.host || "").split(":")[0];
  return origin === "https://itgla.com"
    || origin === "https://www.itgla.com"
    || origin === `https://${host}`
    || origin === `http://${host}`;
}

const feedbackAttempts = new Map();

function allowFeedback(request) {
  const key = clientAddress(request);
  const now = Date.now();
  const attempts = (feedbackAttempts.get(key) || []).filter((time) => now - time < 60 * 60 * 1000);
  if (attempts.length >= 5) return false;
  attempts.push(now);
  feedbackAttempts.set(key, attempts);
  return true;
}

function validEmail(value) {
  if (value == null || value === "") return null;
  if (typeof value !== "string" || value.length > 254 || !/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(value)) return undefined;
  return value;
}

async function sendFeedbackEmail(id, message, replyEmail) {
  if (!config.resendApiKey || !config.feedbackFrom) return { status: "not_configured", providerId: null };
  try {
    const result = await fetch("https://api.resend.com/emails", {
      method: "POST",
      headers: {
        Authorization: `Bearer ${config.resendApiKey}`,
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        from: config.feedbackFrom,
        to: ["service@itgla.com"],
        ...(replyEmail ? { reply_to: replyEmail } : {}),
        subject: `ITGLA website feedback #${id}`,
        text: `Feedback submission #${id}\n\n${message}${replyEmail ? `\n\nReply email: ${replyEmail}` : "\n\nNo reply email provided."}`,
      }),
      signal: AbortSignal.timeout(8000),
    });
    const body = await result.json().catch(() => ({}));
    return result.ok
      ? { status: "sent", providerId: body.id || null }
      : { status: "failed", providerId: null };
  } catch {
    return { status: "failed", providerId: null };
  }
}

async function sendSigninEmail(email, token) {
  if (!config.resendApiKey || !config.feedbackFrom) return { status: "not_configured", providerId: null };
  const verifyUrl = `${config.publicBaseUrl}/auth/email/verify?token=${encodeURIComponent(token)}`;
  try {
    const result = await fetch("https://api.resend.com/emails", {
      method: "POST",
      headers: { Authorization: `Bearer ${config.resendApiKey}`, "Content-Type": "application/json" },
      body: JSON.stringify({
        from: config.feedbackFrom,
        to: [email],
        subject: "Your ITGLA sign-in link",
        text: `Use this one-time link to sign in to ITGLA:\n\n${verifyUrl}\n\nThis link expires in 15 minutes and can only be used once.`,
      }),
      signal: AbortSignal.timeout(8000),
    });
    const body = await result.json().catch(() => ({}));
    return result.ok ? { status: "sent", providerId: body.id || null } : { status: "failed", providerId: null };
  } catch {
    return { status: "failed", providerId: null };
  }
}

function allowLogin(request) {
  const key = clientAddress(request);
  const now = Date.now();
  const attempts = (loginAttempts.get(key) || []).filter((time) => now - time < 15 * 60 * 1000);
  if (attempts.length >= 10) return false;
  attempts.push(now);
  loginAttempts.set(key, attempts);
  return true;
}

let ingestionRun;
function ingestAccessLog() {
  if (!ingestionRun) ingestionRun = ingestAccessLogBatch().finally(() => { ingestionRun = undefined; });
  return ingestionRun;
}

async function ingestAccessLogBatch() {
  let handle;
  try {
    handle = await fs.open(config.logPath, "r");
    const offsetResult = await pool.query(
      "select byte_offset from analytics_log_offsets where source_path = $1",
      [config.logPath],
    );
    const offset = Number(offsetResult.rows[0]?.byte_offset || 0);
    const stat = await handle.stat();
    const start = offset > stat.size ? 0 : offset;
    const stream = handle.createReadStream({ start, encoding: "utf8" });
    let buffer = "";
    let consumed = start;
    for await (const chunk of stream) {
      buffer += chunk;
      const lines = buffer.split("\n");
      buffer = lines.pop() || "";
      for (const line of lines) {
        consumed += Buffer.byteLength(`${line}\n`);
        const event = parseLogLine(line, config.visitorSalt);
        if (!event) continue;
        const geo = event.ipAddress ? await lookupGeo(event.ipAddress, {
          dbPath: config.geoIpDbPath,
          provider: config.geoProvider,
        }) : null;
        const visitorQuality = isMeaningfulVisitor({
          userAgent: event.userAgent,
          remoteAddress: event.ipAddress || "unknown",
        }) ? "meaningful" : "probe";
        const insertValues = [
          event.eventKey,
          event.eventName,
          event.path,
          event.method,
          event.statusCode,
          event.bytesSent,
          event.referrer,
          event.userAgent,
          event.visitorKey,
          visitorQuality,
          event.ipAddress,
          geo?.countryCode,
          geo?.countryName,
          null,
          null,
          null,
          null,
          null,
          null,
          null,
          null,
          null,
          event.occurredAt,
        ];
        if (insertValues.length !== 23) {
          throw new Error(`analytics insert mapping has ${insertValues.length} values; expected 23`);
        }
        await pool.query(
          `insert into analytics_events
            (event_key,event_name,path,method,status_code,bytes_sent,referrer,user_agent,visitor_key,visitor_quality,
            ip_address,country_code,country_name,region,city,latitude,longitude,timezone,asn,organization,isp,geo_source,occurred_at)
           values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21,$22,$23)
           on conflict (event_key) do update set
             event_name=excluded.event_name,
             ip_address=null,
             country_code=coalesce(analytics_events.country_code, excluded.country_code),
             country_name=coalesce(analytics_events.country_name, excluded.country_name),
             region=null, city=null, latitude=null, longitude=null, timezone=null,
             asn=null, organization=null, isp=null, geo_source=null,
             visitor_quality=excluded.visitor_quality`,
          insertValues,
        );
      }
    }
    await pool.query(
      `insert into analytics_log_offsets (source_path,byte_offset) values ($1,$2)
       on conflict (source_path) do update set byte_offset=excluded.byte_offset,updated_at=now()`,
      [config.logPath, consumed],
    );
  } catch (error) {
    console.error("analytics log ingestion failed", error);
    throw error;
  } finally {
    await handle?.close();
  }
}

async function overview(days) {
  const result = await pool.query(
    `select
       coalesce(sum(event_count) filter (where event_name in ('page_view','download_page_view')),0)::int as page_views,
       count(distinct visitor_key) filter (where event_name in ('page_view','download_page_view'))::int as unique_visitors,
       coalesce(sum(event_count) filter (where event_name in ('download_success','download')),0)::int as downloads,
       coalesce(sum(event_count) filter (where event_name='download_page_view'),0)::int as download_page_views,
       count(distinct visitor_key) filter (where event_name='download_page_view')::int as download_page_visitors,
       coalesce(sum(event_count) filter (where event_name='download_failure'),0)::int as download_failures,
       coalesce(sum(event_count) filter (where event_name='page_error'),0)::int as errors,
       max(occurred_at) as latest_event_at
     from itgla_report_events($1) where occurred_at >= now() - ($1::int * interval '1 day')
       and visitor_quality='meaningful'`,
    [days],
  );
  return { days, ...result.rows[0] };
}

async function daily(days) {
  const result = await pool.query(
    `select to_char(date_trunc('day',occurred_at),'YYYY-MM-DD') as day,
       coalesce(sum(event_count) filter (where event_name in ('page_view','download_page_view')),0)::int as page_views,
       count(distinct visitor_key) filter (where event_name in ('page_view','download_page_view'))::int as unique_visitors,
       coalesce(sum(event_count) filter (where event_name in ('download_success','download')),0)::int as downloads,
       coalesce(sum(event_count) filter (where event_name='download_page_view'),0)::int as download_page_views,
       count(distinct visitor_key) filter (where event_name='download_page_view')::int as download_page_visitors,
       count(distinct visitor_key) filter (where event_name in ('download_success','download'))::int as successful_visitors,
       coalesce(sum(event_count) filter (where event_name='download_failure'),0)::int as download_failures,
       coalesce(sum(event_count) filter (where event_name='page_error'),0)::int as errors
     from itgla_report_events($1) where occurred_at >= now() - ($1::int * interval '1 day')
       and visitor_quality='meaningful'
     group by 1 order by 1`,
    [days],
  );
  return result.rows;
}

async function countries(days) {
  const summary = await pool.query(
    `select coalesce(country_name,country_code,'Unknown') as country,
       coalesce(sum(event_count),0)::int as page_views,
       count(distinct visitor_key)::int as unique_visitors
     from itgla_report_events($1)
     where occurred_at >= now() - ($1::int * interval '1 day')
       and visitor_quality='meaningful'
       and event_name in ('page_view','download_page_view')
     group by 1 order by page_views desc, country asc limit 30`,
    [days],
  );
  const dailyResult = await pool.query(
    `select to_char(date_trunc('day',occurred_at),'YYYY-MM-DD') as day,
       coalesce(country_name,country_code,'Unknown') as country,
       coalesce(sum(event_count),0)::int as page_views,
       count(distinct visitor_key)::int as unique_visitors
     from itgla_report_events($1)
     where occurred_at >= now() - ($1::int * interval '1 day')
       and visitor_quality='meaningful'
       and event_name in ('page_view','download_page_view')
     group by 1,2 order by 1, page_views desc`,
    [days],
  );
  return { summary: summary.rows, daily: dailyResult.rows };
}

async function downloadCountries(days) {
  const summary = await pool.query(
    `select coalesce(country_name,country_code,'Unknown') as country,
       coalesce(sum(event_count) filter (where event_name in ('download_success','download')),0)::int as downloads,
       coalesce(sum(event_count) filter (where event_name='download_failure'),0)::int as failures
     from itgla_report_events($1)
     where occurred_at >= now() - ($1::int * interval '1 day')
       and visitor_quality='meaningful'
       and event_name in ('download_success','download_failure','download')
     group by 1 order by downloads desc, failures desc, country asc limit 30`,
    [days],
  );
  const dailyResult = await pool.query(
    `select to_char(date_trunc('day',occurred_at),'YYYY-MM-DD') as day,
       coalesce(country_name,country_code,'Unknown') as country,
       coalesce(sum(event_count) filter (where event_name in ('download_success','download')),0)::int as downloads,
       coalesce(sum(event_count) filter (where event_name='download_failure'),0)::int as failures
     from itgla_report_events($1)
     where occurred_at >= now() - ($1::int * interval '1 day')
       and visitor_quality='meaningful'
       and event_name in ('download_success','download_failure','download')
     group by 1,2 order by 1, downloads desc`,
    [days],
  );
  return { summary: summary.rows, daily: dailyResult.rows };
}

async function pages(days) {
  const result = await pool.query(
    `select path,coalesce(sum(event_count),0)::int as page_views,count(distinct visitor_key)::int as unique_visitors,
       max(occurred_at) as last_seen_at
     from itgla_report_events($1) where occurred_at >= now() - ($1::int * interval '1 day')
       and visitor_quality='meaningful'
       and event_name in ('page_view','download_page_view')
     group by path order by page_views desc,last_seen_at desc limit 100`,
    [days],
  );
  return result.rows;
}

async function recentEvents(days) {
  const result = await pool.query(
    `select event_name,path,status_code,bytes_sent,referrer,occurred_at
     from analytics_events where occurred_at >= now() - ($1::int * interval '1 day')
       and visitor_quality='meaningful'
     order by occurred_at desc limit 100`,
    [days],
  );
  return result.rows;
}

async function downloadEvents(days) {
  const result = await pool.query(
    `select event_name,path,method,status_code,bytes_sent,referrer,user_agent,country_code,country_name,occurred_at
     from analytics_events
     where occurred_at >= now() - ($1::int * interval '1 day')
       and visitor_quality='meaningful'
       and event_name in ('download_page_view','download_success','download_failure','download')
     order by occurred_at desc limit 200`,
    [days],
  );
  return result.rows;
}

async function downloadEventPage(days, cursor = null) {
  const result = await pool.query(
    `select id,event_name,path,method,status_code,bytes_sent,referrer,user_agent,country_code,country_name,occurred_at
     from analytics_events
     where occurred_at >= now() - ($1::int * interval '1 day')
       and visitor_quality='meaningful'
       and event_name in ('download_page_view','download_success','download_failure','download')
       and ($2::bigint is null or (occurred_at,id) <
         (select occurred_at,id from analytics_events where id=$2::bigint))
     order by occurred_at desc,id desc limit 26`,
    [days, cursor],
  );
  const events = result.rows.slice(0, 25);
  return { events, next_cursor: result.rows.length > 25 ? String(events.at(-1).id) : null };
}

async function downloadSummary(days) {
  const summary = await pool.query(
    `with page_visits as (
       select distinct visitor_key
       from itgla_report_events($1)
       where event_name='download_page_view' and visitor_quality='meaningful'
         and occurred_at >= now() - ($1::int * interval '1 day')
     ), successful as (
       select distinct visitor_key
       from itgla_report_events($1)
       where event_name in ('download_success','download') and visitor_quality='meaningful'
         and occurred_at >= now() - ($1::int * interval '1 day')
     )
     select
       (select count(*)::int from page_visits) as page_visitors,
       (select count(*)::int from successful) as successful_visitors,
       (select count(*)::int from page_visits p where not exists (select 1 from successful s where s.visitor_key=p.visitor_key)) as no_download_visitors,
       (select count(distinct visitor_key)::int from itgla_report_events($1) where event_name='download_failure' and visitor_quality='meaningful' and occurred_at >= now() - ($1::int * interval '1 day')) as failed_visitors,
       (select coalesce(sum(event_count),0)::int from itgla_report_events($1) where event_name='download_failure' and visitor_quality='meaningful' and occurred_at >= now() - ($1::int * interval '1 day')) as failures`,
    [days],
  );
  return summary.rows[0];
}

// Retain the existing endpoint for compatibility with already-open admin pages.
async function downloadDetails(days) {
  const [events, summary] = await Promise.all([downloadEvents(days), downloadSummary(days)]);
  return { events, summary };
}

async function recentErrors(days) {
  const result = await pool.query(
    `select event_name,path,method,status_code,bytes_sent,referrer,user_agent,occurred_at
     from analytics_events where occurred_at >= now() - ($1::int * interval '1 day')
       and visitor_quality='meaningful'
       and event_name='page_error'
     order by occurred_at desc limit 100`,
    [days],
  );
  return result.rows;
}

const reportWindows = [7, 30, 90, 365];
let reportRefresh;

async function refreshDailyFacts() {
  const client = await pool.connect();
  try {
    await client.query("begin");
    // Claim dirty dates atomically. Concurrent event writes leave a new dirty
    // marker after this transaction commits, so late events cannot be lost.
    const { rows } = await client.query("delete from analytics_dirty_days returning day::text");
    const dates = new Set(rows.map((row) => row.day));
    dates.add(new Date().toISOString().slice(0, 10));
    for (const day of dates) {
      await client.query("delete from analytics_daily_facts where day=$1::date", [day]);
      await client.query(
        `insert into analytics_daily_facts(day,event_name,path,visitor_key,country,event_count,occurred_at)
         select $1::date,event_name,path,visitor_key,coalesce(country_name,country_code,'Unknown'),count(*),max(occurred_at)
         from analytics_events
         where occurred_at >= ($1::date::timestamp at time zone 'UTC')
           and occurred_at < (($1::date + 1)::timestamp at time zone 'UTC')
           and visitor_quality='meaningful'
         group by event_name,path,visitor_key,coalesce(country_name,country_code,'Unknown')`,
        [day],
      );
    }
    await client.query("commit");
    return [...dates].sort();
  } catch (error) {
    await client.query("rollback");
    throw error;
  } finally { client.release(); }
}

async function refreshReportSnapshots() {
  if (reportRefresh) return reportRefresh;
  reportRefresh = (async () => {
    await ingestAccessLog();
    const refreshedDays = await refreshDailyFacts();
    const snapshots = [];
    for (const days of reportWindows) {
      const reports = await Promise.all([
        ["overview", overview(days)],
        ["daily", daily(days)],
        ["countries", countries(days)],
        ["download-countries", downloadCountries(days)],
        ["pages", pages(days)],
        ["events", recentEvents(days)],
        ["download-summary", downloadSummary(days)],
        ["errors", recentErrors(days)],
      ].map(async ([name, result]) => [name, await result]));
      for (const [name, payload] of reports) snapshots.push({ name, days, payload });
    }

    const client = await pool.connect();
    try {
      await client.query("begin");
      for (const snapshot of snapshots) {
        await client.query(
          `insert into analytics_report_snapshots(report_name,days,payload,computed_at)
           values($1,$2,$3::jsonb,transaction_timestamp())
           on conflict(report_name,days) do update
             set payload=excluded.payload,computed_at=excluded.computed_at`,
          [snapshot.name, snapshot.days, JSON.stringify(snapshot.payload)],
        );
      }
      const { rows } = await client.query("select transaction_timestamp() as computed_at");
      await client.query("commit");
      console.info(JSON.stringify({ event: "analytics_reports_refreshed", refreshedDays, windows: reportWindows, reports: snapshots.length, computedAt: rows[0].computed_at }));
      return rows[0].computed_at;
    } catch (error) {
      await client.query("rollback");
      throw error;
    } finally {
      client.release();
    }
  })().finally(() => { reportRefresh = undefined; });
  return reportRefresh;
}

async function readReportSnapshot(reportName, days) {
  const { rows } = await pool.query(
    "select payload,computed_at from analytics_report_snapshots where report_name=$1 and days=$2",
    [reportName, days],
  );
  if (!rows.length) throw Object.assign(new Error("Statistics are being prepared. Try again shortly."), { status: 503 });
  return rows[0];
}

async function sendReportSnapshot(response, reportName, days) {
  const snapshot = await readReportSnapshot(reportName, days);
  return json(response, 200, snapshot.payload, { "X-Analytics-Computed-At": new Date(snapshot.computed_at).toISOString() });
}

async function handle(request, response) {
  const url = new URL(request.url, `http://${request.headers.host || "localhost"}`);
  const pathname = url.pathname;
  if (adminCredentialRedirect(url)) {
    response.writeHead(303, { ...securityHeaders("text/plain; charset=utf-8"), Location: "/admin" });
    return response.end("Use the sign-in form at /admin.");
  }
  if (request.method === "GET" && pathname === "/healthz") {
    await pool.query("select 1");
    return json(response, 200, { status: "ok" });
  }
  if (request.method === "GET" && pathname === "/feedback.js") return asset(response, "feedback.js", "text/javascript; charset=utf-8");
  if (request.method === "GET" && pathname === "/auth.js") return asset(response, "auth.js", "text/javascript; charset=utf-8");
  if (request.method === "GET" && pathname === "/account.js") return asset(response, "account.js", "text/javascript; charset=utf-8");
  if (request.method === "GET" && pathname === "/auth/email/verify") {
    const token = url.searchParams.get("token") || "";
    if (!/^[a-f0-9]{64}$/.test(token)) return asset(response, "auth-invalid.html", "text/html; charset=utf-8");
    const result = await pool.query(
      "select email,request_id from email_signin_tokens where token_hash=$1 and used_at is null and expires_at > now()",
      [hash(token)],
    );
    if (!result.rowCount) return asset(response, "auth-invalid.html", "text/html; charset=utf-8");
    const session = crypto.randomBytes(32).toString("hex");
    await pool.query(
      "insert into users (email) values ($1) on conflict (email) do update set last_sign_in_at=now()",
      [result.rows[0].email],
    );
    await pool.query("update email_signin_tokens set used_at=now() where token_hash=$1", [hash(token)]);
    if (result.rows[0].request_id) {
      await pool.query("update email_login_attempts set verified_at=now() where request_id=$1", [result.rows[0].request_id]);
    }
    await pool.query(
      "insert into user_sessions (token_hash,email,expires_at) values ($1,$2,now()+interval '30 days')",
      [hash(session), result.rows[0].email],
    );
    response.writeHead(302, {
      Location: "/account.html?registered=1",
      "Set-Cookie": `itgla_user_session=${session}; Path=/; Max-Age=2592000; HttpOnly; Secure; SameSite=Lax`,
      ...securityHeaders("text/plain; charset=utf-8"),
    });
    return response.end();
  }
  if (request.method === "GET" && pathname === "/auth/me") {
    const email = await userEmail(request);
    return json(response, 200, email ? { authenticated: true, email } : { authenticated: false });
  }
  if (request.method === "POST" && pathname === "/auth/logout") {
    if (!sameOrigin(request)) return json(response, 403, { message: "Origin not allowed." });
    const token = userSessionToken(request);
    if (token) await pool.query("delete from user_sessions where token_hash=$1", [hash(token)]);
    return json(response, 200, { signed_out: true }, {
      "Set-Cookie": "itgla_user_session=; Path=/; Max-Age=0; HttpOnly; Secure; SameSite=Lax",
    });
  }
  if (request.method === "GET" && pathname === "/auth/email/status") {
    const requestId = url.searchParams.get("request_id") || "";
    if (!/^[a-f0-9]{32}$/.test(requestId)) return json(response, 400, { message: "Invalid sign-in attempt." });
    const attempt = await pool.query(
      "select email,verified_at from email_login_attempts where request_id=$1 and expires_at > now()",
      [requestId],
    );
    if (!attempt.rowCount || !attempt.rows[0].verified_at) return json(response, 202, { authenticated: false, status: "pending" });
    const session = crypto.randomBytes(32).toString("hex");
    await pool.query(
      "insert into user_sessions (token_hash,email,expires_at) values ($1,$2,now()+interval '30 days')",
      [hash(session), attempt.rows[0].email],
    );
    return json(response, 200, { authenticated: true }, {
      "Set-Cookie": `itgla_user_session=${session}; Path=/; Max-Age=2592000; HttpOnly; Secure; SameSite=Lax`,
    });
  }
  if (pathname.startsWith("/api/workspace/")) {
    const email = await userEmail(request);
    if (!email) return json(response, 401, { message: "Sign-in required." });
    if (request.method === "GET" && pathname === "/api/workspace/servers") {
      const result = await pool.query(
        "select id,name,tags,ip_address,ports,created_at from user_servers where email=$1 order by updated_at desc,id desc",
        [email],
      );
      return json(response, 200, result.rows);
    }
    if (request.method === "POST" && pathname === "/api/workspace/servers") {
      if (!sameOrigin(request)) return json(response, 403, { message: "Origin not allowed." });
      const payload = await readBody(request).catch(() => null);
      const fields = ["name", "tags", "ip_address", "ports"];
      if (!payload || typeof payload.name !== "string" || !payload.name.trim() || payload.name.length > 160) {
        return json(response, 400, { message: "Server name is required and must be 160 characters or fewer." });
      }
      if (fields.some((field) => field !== "name" && typeof payload[field] !== "string" || (typeof payload[field] === "string" && payload[field].length > 2000))) {
        return json(response, 400, { message: "Server fields are invalid." });
      }
      const result = await pool.query(
        "insert into user_servers (email,name,tags,ip_address,ports) values ($1,$2,$3,$4,$5) returning id,name,tags,ip_address,ports,created_at",
        [email, payload.name.trim(), payload.tags.trim(), payload.ip_address.trim(), payload.ports.trim()],
      );
      return json(response, 201, result.rows[0]);
    }
    const updateMatch = pathname.match(/^\/api\/workspace\/servers\/(\d+)$/);
    if (request.method === "PATCH" && updateMatch) {
      if (!sameOrigin(request)) return json(response, 403, { message: "Origin not allowed." });
      const payload = await readBody(request).catch(() => null);
      const fields = ["name", "tags", "ip_address", "ports"];
      if (!payload || typeof payload.name !== "string" || !payload.name.trim() || payload.name.length > 160) {
        return json(response, 400, { message: "Server name is required and must be 160 characters or fewer." });
      }
      if (fields.some((field) => field !== "name" && typeof payload[field] !== "string" || (typeof payload[field] === "string" && payload[field].length > 2000))) {
        return json(response, 400, { message: "Server fields are invalid." });
      }
      const result = await pool.query(
        "update user_servers set name=$1,tags=$2,ip_address=$3,ports=$4,updated_at=now() where id=$5 and email=$6 returning id,name,tags,ip_address,ports,created_at,updated_at",
        [payload.name.trim(), payload.tags.trim(), payload.ip_address.trim(), payload.ports.trim(), updateMatch[1], email],
      );
      return result.rowCount ? json(response, 200, result.rows[0]) : json(response, 404, { message: "Server not found." });
    }
    const deleteMatch = pathname.match(/^\/api\/workspace\/servers\/(\d+)$/);
    if (request.method === "DELETE" && deleteMatch) {
      if (!sameOrigin(request)) return json(response, 403, { message: "Origin not allowed." });
      const result = await pool.query(
        "delete from user_servers where id=$1 and email=$2 returning id",
        [deleteMatch[1], email],
      );
      return result.rowCount ? json(response, 200, { deleted: true }) : json(response, 404, { message: "Server not found." });
    }
  }
  if (request.method === "POST" && pathname === "/auth/email/start") {
    if (!sameOrigin(request)) return json(response, 403, { message: "Origin not allowed." });
    const payload = await readBody(request).catch(() => null);
    const email = payload && validEmail(payload.email);
    if (email === undefined || !email) return json(response, 400, { message: "Enter a valid email address." });
    if (!config.resendApiKey) return json(response, 503, { message: "Email delivery is temporarily unavailable. Please try again later." });
    const token = crypto.randomBytes(32).toString("hex");
    const requestId = crypto.randomBytes(16).toString("hex");
    await pool.query(
      "insert into email_login_attempts (request_id,email,expires_at) values ($1,$2,now()+interval '15 minutes')",
      [requestId, email],
    );
    await pool.query(
      "insert into email_signin_tokens (token_hash,email,request_id,expires_at) values ($1,$2,$3,now()+interval '15 minutes')",
      [hash(token), email, requestId],
    );
    const delivery = await sendSigninEmail(email, token);
    if (delivery.status !== "sent") {
      await pool.query("delete from email_signin_tokens where token_hash=$1", [hash(token)]);
      await pool.query("delete from email_login_attempts where request_id=$1", [requestId]);
      return json(response, 502, { message: "The verification email could not be sent. Please try again." });
    }
    return json(response, 202, { message: "Check your inbox for a one-time ITGLA sign-in link.", request_id: requestId });
  }
  if (request.method === "POST" && pathname === "/api/feedback") {
    if (!sameOrigin(request)) return json(response, 403, { message: "Origin not allowed." });
    if (!allowFeedback(request)) return json(response, 429, { message: "Too many feedback submissions. Please try again later." }, { "Retry-After": "3600" });
    const payload = await readBody(request).catch(() => null);
    if (!payload) return json(response, 400, { message: "Feedback could not be read. Please try again." });
    if (typeof payload.website === "string" && payload.website.trim()) return json(response, 202, { status: "accepted" });
    const message = typeof payload.message === "string" ? payload.message.trim() : "";
    const replyEmail = validEmail(payload.email);
    if (message.length < 5 || message.length > 5000) return json(response, 400, { message: "Feedback must be between 5 and 5,000 characters." });
    if (replyEmail === undefined) return json(response, 400, { message: "Enter a valid email address or leave it blank." });
    const inserted = await pool.query(
      "insert into feedback_submissions (message,reply_email) values ($1,$2) returning id",
      [message, replyEmail],
    );
    const id = inserted.rows[0].id;
    const delivery = await sendFeedbackEmail(id, message, replyEmail);
    await pool.query(
      "update feedback_submissions set email_status=$2,email_provider_id=$3 where id=$1",
      [id, delivery.status, delivery.providerId],
    );
    return json(response, 201, { status: "received", email_status: delivery.status });
  }
  if (request.method === "GET" && (pathname === "/admin" || pathname === "/admin/")) return asset(response, "admin.html", "text/html; charset=utf-8");
  if (request.method === "GET" && pathname === "/admin/feedback.js") return asset(response, "feedback-admin.js", "text/javascript; charset=utf-8");
  if (request.method === "GET" && pathname === "/admin/admin.css") return asset(response, "admin.css", "text/css; charset=utf-8");
  if (request.method === "GET" && pathname === "/admin/admin.js") return asset(response, "admin.js", "text/javascript; charset=utf-8");
  if (request.method === "GET" && pathname === "/admin/chart.umd.min.js") return asset(response, "chart.umd.min.js", "text/javascript; charset=utf-8");

  if (request.method === "POST" && !sameOrigin(request)) return json(response, 403, { message: "Origin not allowed." });

  if (request.method === "POST" && pathname === "/admin/api/login") {
    if (!allowLogin(request)) return json(response, 429, { message: "Too many sign-in attempts. Try again later." }, { "Retry-After": "900" });
    const payload = await readBody(request).catch(() => null);
    const supplied = payload ? passwordDigest(payload.password || "", config.adminPasswordSalt) : "";
    if (!payload || String(payload.username || "").trim() !== config.adminUsername || !safeEqual(supplied, config.adminPasswordScrypt)) {
      return json(response, 401, { message: "Incorrect username or password." });
    }
    const token = crypto.randomBytes(32).toString("hex");
    await pool.query(
      "insert into admin_sessions (token_hash,username,expires_at) values ($1,$2,now()+($3::int * interval '1 second'))",
      [hash(token), config.adminUsername, sessionTtlSeconds],
    );
    return json(response, 200, { status: "ok" }, {
      "Set-Cookie": `itgla_admin_session=${token}; Path=/admin; Max-Age=${sessionTtlSeconds}; HttpOnly; Secure; SameSite=Strict`,
    });
  }

  if (request.method === "POST" && pathname === "/admin/api/logout") {
    const token = sessionToken(request);
    if (token) await pool.query("delete from admin_sessions where token_hash=$1", [hash(token)]);
    return json(response, 200, { status: "ok" }, {
      "Set-Cookie": "itgla_admin_session=; Path=/admin; Max-Age=0; HttpOnly; Secure; SameSite=Strict",
    });
  }

  if (request.method === "GET" && pathname === "/admin/api/me") {
    const username = await authenticated(request);
    return json(response, 200, { authenticated: Boolean(username), username });
  }

  if (pathname.startsWith("/admin/api/")) {
    if (!(await authenticated(request))) return json(response, 401, { message: "Administrator sign-in required." });
    if (request.method === "POST" && pathname === "/admin/api/refresh") {
      const computedAt = await refreshReportSnapshots();
      return json(response, 200, { status: "ok", computed_at: computedAt });
    }
    const days = daysFromUrl(url);
    if (pathname === "/admin/api/overview") return sendReportSnapshot(response, "overview", days);
    if (pathname === "/admin/api/daily") return sendReportSnapshot(response, "daily", days);
    if (pathname === "/admin/api/countries") return sendReportSnapshot(response, "countries", days);
    if (pathname === "/admin/api/download-countries") return sendReportSnapshot(response, "download-countries", days);
    if (pathname === "/admin/api/pages") return sendReportSnapshot(response, "pages", days);
    if (pathname === "/admin/api/events") return sendReportSnapshot(response, "events", days);
    if (pathname === "/admin/api/download-summary") return sendReportSnapshot(response, "download-summary", days);
    if (pathname === "/admin/api/errors") return sendReportSnapshot(response, "errors", days);
    if (pathname === "/admin/api/download-events-page") {
      const cursor = url.searchParams.get("cursor");
      if (cursor !== null && (!/^[1-9][0-9]{0,18}$/.test(cursor) || BigInt(cursor) > 9223372036854775807n)) {
        return json(response, 400, { message: "Invalid download page cursor." });
      }
      return json(response, 200, await downloadEventPage(days, cursor));
    }
    if (pathname === "/admin/api/download-events") return json(response, 200, await downloadEvents(days));
    if (pathname === "/admin/api/downloads") return json(response, 200, await downloadDetails(days));
    if (pathname === "/admin/api/feedback" && request.method === "GET") {
      const result = await pool.query(
        `select id,message,reply_email,email_status,created_at,read_at
         from feedback_submissions order by created_at desc limit 200`,
      );
      return json(response, 200, result.rows);
    }
    const feedbackReadMatch = pathname.match(/^\/admin\/api\/feedback\/(\d+)\/read$/);
    if (feedbackReadMatch && request.method === "POST") {
      const result = await pool.query(
        "update feedback_submissions set read_at=coalesce(read_at,now()) where id=$1 returning id,read_at",
        [feedbackReadMatch[1]],
      );
      if (!result.rowCount) return json(response, 404, { message: "Feedback not found." });
      return json(response, 200, result.rows[0]);
    }
  }
  return json(response, 404, { message: "Not found." });
}

await pool.query(await fs.readFile(path.join(root, "schema.sql"), "utf8"));
await pool.query(await fs.readFile(path.join(root, "rollups.sql"), "utf8"));
await pool.query(
  `update analytics_events set ip_address=null, region=null, city=null, latitude=null,
     longitude=null, timezone=null, asn=null, organization=null, isp=null, geo_source=null
   where ip_address is not null or region is not null or city is not null or latitude is not null
      or longitude is not null or timezone is not null or asn is not null or organization is not null
      or isp is not null or geo_source is not null`,
);
await pool.query(
  "delete from analytics_events where event_name in ('page_view','page_error') and not (path = any($1::text[]))",
  [publicPagePaths],
);
await pool.query(
  `update analytics_events set visitor_quality='probe'
   where visitor_quality <> 'probe' and (user_agent is null or btrim(user_agent) = '')`,
);
await pool.query(
  `update analytics_events set visitor_quality='probe'
   where visitor_quality <> 'probe' and lower(user_agent) ~ '(curl|wget|go-http-client|python|node([./ -]|$)|java|okhttp|opsprobe|bot|crawler|spider|scraper|ahrefs|semrush|dataforseo|seranking|gptbot|chatgpt-user|claudebot|anthropic-ai|amazonbot|censys|nutch|cms[- ]checker|webapp[- ]mapper)'`,
);
await pool.query("delete from admin_sessions where expires_at <= now()");
await ingestAccessLog();
await refreshReportSnapshots();
setInterval(() => {
  void ingestAccessLog().catch(() => {}); // The ingestion function logs failure; keep the last good snapshot.
}, 10_000).unref();
setInterval(() => {
  void refreshReportSnapshots().catch((error) => console.error("analytics report refresh failed", error));
}, 60_000).unref();
setInterval(() => {
  void pool.query("delete from admin_sessions where expires_at <= now()");
  void pool.query(
    `update analytics_events set
       ip_address=null, country_code=null, country_name=null, region=null, city=null,
       latitude=null, longitude=null, timezone=null, asn=null, organization=null, isp=null, geo_source=null
     where occurred_at < now() - ($1::int * interval '1 day')
       and event_name in ('page_view','download_page_view','download_success','download_failure','download')`,
    [config.geoRetentionDays],
  );
  void pool.query("delete from analytics_events where occurred_at < now() - ($1::int * interval '1 day')", [config.retentionDays]);
}, 60 * 60 * 1000).unref();

const server = http.createServer((request, response) => {
  if (request.url.startsWith("/admin/api/")) {
    response.adminStartedAt = performance.now();
    response.once("close", () => {
      const durationMs = Math.round(performance.now() - response.adminStartedAt);
      if (durationMs >= 1000 || !response.writableFinished) {
        console.warn(JSON.stringify({ event: "admin_request_timing", path: request.url.split("?")[0].slice(0, 100), status: response.statusCode, durationMs, completed: response.writableFinished, poolWaiting: pool.waitingCount, poolTotal: pool.totalCount }));
      }
    });
  }
  handle(request, response).catch((error) => {
    console.error("request failed", error);
    if (!response.headersSent) json(response, 500, { message: "Internal server error." });
  });
});
server.listen(config.port, "127.0.0.1", () => console.log(`ITGLA analytics listening on 127.0.0.1:${config.port}`));

async function shutdown() {
  server.close();
  await pool.end();
}
process.on("SIGTERM", () => void shutdown());
process.on("SIGINT", () => void shutdown());
