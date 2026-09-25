import fs from "node:fs/promises";
import http from "node:http";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../server/analytics/web");
const sessionCookie = "itgla_admin_session=test-session";
const fixtures = {
  "/admin/api/overview": { days: 30, page_views: 1284, unique_visitors: 462, downloads: 87, errors: 3, latest_event_at: "2026-09-23T14:25:00Z" },
  "/admin/api/daily": [
    { day: "2026-09-20", page_views: 318, unique_visitors: 126, downloads: 18, errors: 1 },
    { day: "2026-09-21", page_views: 402, unique_visitors: 143, downloads: 24, errors: 0 },
    { day: "2026-09-22", page_views: 564, unique_visitors: 193, downloads: 45, errors: 2 },
  ],
  "/admin/api/pages": [
    { path: "/", page_views: 711, unique_visitors: 322 },
    { path: "/downloads.html", page_views: 381, unique_visitors: 211 },
  ],
  "/admin/api/events": [
    { occurred_at: "2026-09-25T14:25:00Z", event_name: "download", path: "/downloads/v0.2.1/itgla.zip", status_code: 200, referrer: "https://itgla.com/downloads.html" },
    { occurred_at: "2026-09-23T14:23:00Z", event_name: "page_view", path: "/", status_code: 200, referrer: "-" },
  ],
};

function json(response, status, body, headers = {}) {
  response.writeHead(status, { "Content-Type": "application/json", ...headers });
  response.end(JSON.stringify(body));
}

const server = http.createServer(async (request, response) => {
  const url = new URL(request.url, "http://127.0.0.1:4174");
  if (request.method === "POST" && url.pathname === "/admin/api/login") {
    let body = "";
    for await (const chunk of request) body += chunk;
    const credentials = JSON.parse(body || "{}");
    if (credentials.username !== "admin" || credentials.password !== "test-password") return json(response, 401, { message: "Incorrect username or password." });
    return json(response, 200, { status: "ok" }, { "Set-Cookie": `${sessionCookie}; Path=/admin; HttpOnly; SameSite=Strict` });
  }
  if (request.method === "POST" && url.pathname === "/admin/api/logout") return json(response, 200, { status: "ok" });
  const authenticated = String(request.headers.cookie || "").includes(sessionCookie);
  if (url.pathname === "/admin/api/me") return json(response, 200, { authenticated, username: authenticated ? "admin" : null });
  const fixtureKey = Object.keys(fixtures).find((key) => url.pathname === key);
  if (fixtureKey) return authenticated ? json(response, 200, fixtures[fixtureKey]) : json(response, 401, { message: "Administrator sign-in required." });

  const assets = {
    "/admin": ["admin.html", "text/html; charset=utf-8"],
    "/admin/": ["admin.html", "text/html; charset=utf-8"],
    "/admin/admin.css": ["admin.css", "text/css; charset=utf-8"],
    "/admin/admin.js": ["admin.js", "text/javascript; charset=utf-8"],
  };
  const asset = assets[url.pathname];
  if (!asset) return json(response, 404, { message: "Not found." });
  response.writeHead(200, { "Content-Type": asset[1] });
  response.end(await fs.readFile(path.join(root, asset[0])));
});

server.listen(4174, "127.0.0.1");
