import crypto from "node:crypto";
import fs from "node:fs/promises";
import http from "node:http";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { Pool } from "pg";
import { daysFromUrl, hash, parseLogLine, passwordDigest, safeEqual } from "./lib.mjs";

const root = path.dirname(fileURLToPath(import.meta.url));
const config = {
  port: Number(process.env.ITGLA_ANALYTICS_PORT || 3420),
  logPath: process.env.ITGLA_ACCESS_LOG || "/var/log/itgla/nginx-access.log",
  databaseUrl: process.env.ITGLA_DATABASE_URL,
  adminUsername: process.env.ITGLA_ADMIN_USERNAME,
  adminPasswordSalt: process.env.ITGLA_ADMIN_PASSWORD_SALT,
  adminPasswordScrypt: process.env.ITGLA_ADMIN_PASSWORD_SCRYPT,
  visitorSalt: process.env.ITGLA_VISITOR_SALT,
  retentionDays: Math.max(30, Number(process.env.ITGLA_RETENTION_DAYS || 365)),
};

for (const [name, value] of Object.entries(config)) {
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
  return !origin || origin === "https://itgla.com";
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
        await pool.query(
          `insert into analytics_events
            (event_key,event_name,path,method,status_code,bytes_sent,referrer,user_agent,visitor_key,occurred_at)
           values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
           on conflict (event_key) do nothing`,
          [
            event.eventKey, event.eventName, event.path, event.method, event.statusCode,
            event.bytesSent, event.referrer, event.userAgent, event.visitorKey, event.occurredAt,
          ],
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
       count(*) filter (where event_name='page_view')::int as page_views,
       count(distinct visitor_key) filter (where event_name='page_view')::int as unique_visitors,
       count(*) filter (where event_name='download')::int as downloads,
       count(*) filter (where event_name='page_error')::int as errors,
       max(occurred_at) as latest_event_at
     from analytics_events where occurred_at >= now() - ($1::int * interval '1 day')`,
    [days],
  );
  return { days, ...result.rows[0] };
}

async function daily(days) {
  const result = await pool.query(
    `select to_char(date_trunc('day',occurred_at),'YYYY-MM-DD') as day,
       count(*) filter (where event_name='page_view')::int as page_views,
       count(distinct visitor_key) filter (where event_name='page_view')::int as unique_visitors,
       count(*) filter (where event_name='download')::int as downloads,
       count(*) filter (where event_name='page_error')::int as errors
     from analytics_events where occurred_at >= now() - ($1::int * interval '1 day')
     group by 1 order by 1`,
    [days],
  );
  return result.rows;
}

async function pages(days) {
  const result = await pool.query(
    `select path,count(*)::int as page_views,count(distinct visitor_key)::int as unique_visitors,
       max(occurred_at) as last_seen_at
     from analytics_events where occurred_at >= now() - ($1::int * interval '1 day')
       and event_name='page_view'
     group by path order by page_views desc,last_seen_at desc limit 100`,
    [days],
  );
  return result.rows;
}

async function recentEvents(days) {
  const result = await pool.query(
    `select event_name,path,status_code,bytes_sent,referrer,occurred_at
     from analytics_events where occurred_at >= now() - ($1::int * interval '1 day')
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
  if (request.method === "GET" && (pathname === "/admin" || pathname === "/admin/")) return asset(response, "admin.html", "text/html; charset=utf-8");
  if (request.method === "GET" && pathname === "/admin/admin.css") return asset(response, "admin.css", "text/css; charset=utf-8");
  if (request.method === "GET" && pathname === "/admin/admin.js") return asset(response, "admin.js", "text/javascript; charset=utf-8");

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
    if (pathname === "/admin/api/pages") return json(response, 200, await pages(days));
    if (pathname === "/admin/api/events") return json(response, 200, await recentEvents(days));
  }
  return json(response, 404, { message: "Not found." });
}

await pool.query(await fs.readFile(path.join(root, "schema.sql"), "utf8"));
await pool.query("delete from admin_sessions where expires_at <= now()");
await ingestAccessLog();
setInterval(() => void ingestAccessLog(), 10_000).unref();
setInterval(() => {
  void pool.query("delete from admin_sessions where expires_at <= now()");
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
