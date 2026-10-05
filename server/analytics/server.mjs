import crypto from "node:crypto";
import fs from "node:fs/promises";
import http from "node:http";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { Pool } from "pg";
import { daysFromUrl, hash, isMeaningfulVisitor, parseLogLine, passwordDigest, publicPagePaths, safeEqual } from "./lib.mjs";
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

async function ingestAccessLog() {
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
    if (error.code !== "ENOENT") console.error("analytics log ingestion failed", error);
  } finally {
    await handle?.close();
  }
}

async function overview(days) {
  const result = await pool.query(
    `select
       count(*) filter (where event_name in ('page_view','download_page_view'))::int as page_views,
       count(distinct visitor_key) filter (where event_name in ('page_view','download_page_view'))::int as unique_visitors,
       count(*) filter (where event_name in ('download_success','download'))::int as downloads,
       count(*) filter (where event_name='download_page_view')::int as download_page_views,
       count(distinct visitor_key) filter (where event_name='download_page_view')::int as download_page_visitors,
       count(*) filter (where event_name='download_failure')::int as download_failures,
       count(*) filter (where event_name='page_error')::int as errors,
       max(occurred_at) as latest_event_at
     from analytics_events where occurred_at >= now() - ($1::int * interval '1 day')
       and visitor_quality='meaningful'`,
    [days],
  );
  return { days, ...result.rows[0] };
}

async function daily(days) {
  const result = await pool.query(
    `select to_char(date_trunc('day',occurred_at),'YYYY-MM-DD') as day,
       count(*) filter (where event_name in ('page_view','download_page_view'))::int as page_views,
       count(distinct visitor_key) filter (where event_name in ('page_view','download_page_view'))::int as unique_visitors,
       count(*) filter (where event_name in ('download_success','download'))::int as downloads,
       count(*) filter (where event_name='download_page_view')::int as download_page_views,
       count(*) filter (where event_name='download_failure')::int as download_failures,
       count(*) filter (where event_name='page_error')::int as errors
     from analytics_events where occurred_at >= now() - ($1::int * interval '1 day')
       and visitor_quality='meaningful'
     group by 1 order by 1`,
    [days],
  );
  return result.rows;
}

async function countries(days) {
  const summary = await pool.query(
    `select coalesce(country_name,country_code,'Unknown') as country,
       count(*)::int as page_views,
       count(distinct visitor_key)::int as unique_visitors
     from analytics_events
     where occurred_at >= now() - ($1::int * interval '1 day')
       and visitor_quality='meaningful'
       and event_name in ('page_view','download_page_view')
     group by 1 order by page_views desc, country asc limit 30`,
    [days],
  );
  const dailyResult = await pool.query(
    `select to_char(date_trunc('day',occurred_at),'YYYY-MM-DD') as day,
       coalesce(country_name,country_code,'Unknown') as country,
       count(*)::int as page_views,
       count(distinct visitor_key)::int as unique_visitors
     from analytics_events
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
       count(*) filter (where event_name in ('download_success','download'))::int as downloads,
       count(*) filter (where event_name='download_failure')::int as failures
     from analytics_events
     where occurred_at >= now() - ($1::int * interval '1 day')
       and visitor_quality='meaningful'
       and event_name in ('download_success','download_failure','download')
     group by 1 order by downloads desc, failures desc, country asc limit 30`,
    [days],
  );
  const dailyResult = await pool.query(
    `select to_char(date_trunc('day',occurred_at),'YYYY-MM-DD') as day,
       coalesce(country_name,country_code,'Unknown') as country,
       count(*) filter (where event_name in ('download_success','download'))::int as downloads,
       count(*) filter (where event_name='download_failure')::int as failures
     from analytics_events
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
    `select path,count(*)::int as page_views,count(distinct visitor_key)::int as unique_visitors,
       max(occurred_at) as last_seen_at
     from analytics_events where occurred_at >= now() - ($1::int * interval '1 day')
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

async function downloadDetails(days) {
  const result = await pool.query(
    `select event_name,path,method,status_code,bytes_sent,referrer,user_agent,country_code,country_name,occurred_at
     from analytics_events
     where occurred_at >= now() - ($1::int * interval '1 day')
       and visitor_quality='meaningful'
       and event_name in ('download_page_view','download_success','download_failure','download')
     order by occurred_at desc limit 200`,
    [days],
  );
  const summary = await pool.query(
    `with page_visits as (
       select distinct visitor_key
       from analytics_events
       where event_name='download_page_view' and visitor_quality='meaningful'
         and occurred_at >= now() - ($1::int * interval '1 day')
     ), successful as (
       select distinct visitor_key
       from analytics_events
       where event_name in ('download_success','download') and visitor_quality='meaningful'
         and occurred_at >= now() - ($1::int * interval '1 day')
     )
     select
       (select count(*)::int from page_visits) as page_visitors,
       (select count(*)::int from successful) as successful_visitors,
       (select count(*)::int from page_visits p where not exists (select 1 from successful s where s.visitor_key=p.visitor_key)) as no_download_visitors,
       (select count(*)::int from analytics_events where event_name='download_failure' and visitor_quality='meaningful' and occurred_at >= now() - ($1::int * interval '1 day')) as failures`,
    [days],
  );
  return { events: result.rows, summary: summary.rows[0] };
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

async function handle(request, response) {
  const url = new URL(request.url, `http://${request.headers.host || "localhost"}`);
  const pathname = url.pathname;
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
      "select email from email_signin_tokens where token_hash=$1 and used_at is null and expires_at > now()",
      [hash(token)],
    );
    if (!result.rowCount) return asset(response, "auth-invalid.html", "text/html; charset=utf-8");
    const session = crypto.randomBytes(32).toString("hex");
    await pool.query(
      "insert into users (email) values ($1) on conflict (email) do update set last_sign_in_at=now()",
      [result.rows[0].email],
    );
    await pool.query("update email_signin_tokens set used_at=now() where token_hash=$1", [hash(token)]);
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
    const token = cookieValue(request, "itgla_user_session");
    if (!token) return json(response, 200, { authenticated: false });
    const result = await pool.query(
      "select u.email from user_sessions s join users u on u.email=s.email where s.token_hash=$1 and s.expires_at > now()",
      [hash(token)],
    );
    return json(response, 200, result.rowCount
      ? { authenticated: true, email: result.rows[0].email }
      : { authenticated: false });
  }
  if (request.method === "POST" && pathname === "/auth/email/start") {
    if (!sameOrigin(request)) return json(response, 403, { message: "Origin not allowed." });
    const payload = await readBody(request).catch(() => null);
    const email = payload && validEmail(payload.email);
    if (email === undefined || !email) return json(response, 400, { message: "Enter a valid email address." });
    if (!config.resendApiKey) return json(response, 503, { message: "Email delivery is temporarily unavailable. Please try again later." });
    const token = crypto.randomBytes(32).toString("hex");
    await pool.query(
      "insert into email_signin_tokens (token_hash,email,expires_at) values ($1,$2,now()+interval '15 minutes')",
      [hash(token), email],
    );
    const delivery = await sendSigninEmail(email, token);
    if (delivery.status !== "sent") {
      await pool.query("delete from email_signin_tokens where token_hash=$1", [hash(token)]);
      return json(response, 502, { message: "The verification email could not be sent. Please try again." });
    }
    return json(response, 202, { message: "Check your inbox for a one-time ITGLA sign-in link." });
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
    const days = daysFromUrl(url);
    if (pathname === "/admin/api/overview") return json(response, 200, await overview(days));
    if (pathname === "/admin/api/daily") return json(response, 200, await daily(days));
    if (pathname === "/admin/api/countries") return json(response, 200, await countries(days));
    if (pathname === "/admin/api/download-countries") return json(response, 200, await downloadCountries(days));
    if (pathname === "/admin/api/pages") return json(response, 200, await pages(days));
    if (pathname === "/admin/api/events") return json(response, 200, await recentEvents(days));
    if (pathname === "/admin/api/downloads") return json(response, 200, await downloadDetails(days));
    if (pathname === "/admin/api/errors") return json(response, 200, await recentErrors(days));
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
await pool.query(
  `update analytics_events set ip_address=null, region=null, city=null, latitude=null,
     longitude=null, timezone=null, asn=null, organization=null, isp=null, geo_source=null`,
);
await pool.query(
  "delete from analytics_events where event_name in ('page_view','page_error') and not (path = any($1::text[]))",
  [publicPagePaths],
);
await pool.query(
  `update analytics_events set visitor_quality='probe'
   where user_agent is null or btrim(user_agent) = ''`,
);
await pool.query(
  `update analytics_events set visitor_quality='probe'
   where lower(user_agent) ~ '(curl|wget|go-http-client|python|node([./ -]|$)|java|okhttp|opsprobe|bot|crawler|spider|scraper|ahrefs|semrush|dataforseo|seranking|gptbot|chatgpt-user|claudebot|anthropic-ai|amazonbot|censys|nutch|cms[- ]checker|webapp[- ]mapper)'`,
);
await pool.query("delete from admin_sessions where expires_at <= now()");
await ingestAccessLog();
setInterval(() => void ingestAccessLog(), 10_000).unref();
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
