import crypto from "node:crypto";

export const publicPagePaths = Object.freeze([
  "/",
  "/index.html",
  "/downloads.html",
  "/privacy.html",
  "/legal.html",
  "/refund.html",
  "/cookies.html",
  "/feedback.html",
]);

const probeUserAgentPattern = /(curl|wget|go-http-client|python|node(?:\.js)?|java|okhttp|opsprobe|bot|crawler|spider|scraper|ahrefs|semrush|dataforseo|seranking|gptbot|chatgpt-user|claudebot|anthropic-ai|amazonbot|censys|nutch|cms[- ]checker|webapp[- ]mapper)/i;

export function isPrivateOrLoopbackAddress(value) {
  const address = String(value || "").trim().replace(/^\[|\]$/g, "");
  if (!address || address === "unknown" || address === "-" || address === "::") return true;
  if (/^127\./.test(address) || /^10\./.test(address) || /^192\.168\./.test(address) || /^169\.254\./.test(address)) return true;
  const ipv4 = address.match(/^172\.(\d+)\./);
  if (ipv4 && Number(ipv4[1]) >= 16 && Number(ipv4[1]) <= 31) return true;
  if (/^fc[0-9a-f]{2}:|^fd[0-9a-f]{2}:|^fe80:/i.test(address)) return true;
  return false;
}

export function isMeaningfulVisitor({ userAgent, remoteAddress }) {
  const normalizedAgent = String(userAgent || "").trim();
  return Boolean(normalizedAgent) && !probeUserAgentPattern.test(normalizedAgent) && !isPrivateOrLoopbackAddress(remoteAddress);
}

export function hash(value) {
  return crypto.createHash("sha256").update(value).digest("hex");
}

export function safeEqual(left, right) {
  const a = Buffer.from(String(left));
  const b = Buffer.from(String(right));
  return a.length === b.length && crypto.timingSafeEqual(a, b);
}

export function passwordDigest(password, salt) {
  return crypto.scryptSync(String(password), String(salt), 64).toString("hex");
}

export function parseRequestLine(requestLine) {
  const match = /^([A-Z]+)\s+(\S+)\s+HTTP\/\d(?:\.\d)?$/.exec(requestLine || "");
  if (!match) return null;
  const parsed = new URL(match[2], "https://itgla.local");
  return { method: match[1], path: parsed.pathname };
}

export function eventName({ path, contentType, statusCode }) {
  if (path.startsWith("/admin") || path === "/healthz" || path.startsWith("/.well-known/")) return null;
  if (path.startsWith("/downloads/")) return statusCode >= 400 ? "download_failure" : "download_success";
  if (publicPagePaths.includes(path) && contentType?.startsWith("text/html")) {
    if (path === "/downloads.html" && statusCode < 400) return "download_page_view";
    return statusCode >= 400 ? "page_error" : "page_view";
  }
  return null;
}

export function sanitizeReferrer(value) {
  const referrer = String(value || "").trim();
  if (!referrer || referrer === "-") return null;
  try {
    const parsed = new URL(referrer);
    return `${parsed.protocol}//${parsed.host}${parsed.pathname}`.slice(0, 1000);
  } catch {
    return null;
  }
}

export function parseLogLine(line, visitorSalt) {
  try {
    const entry = JSON.parse(line);
    const parsed = parseRequestLine(entry.request);
    if (!parsed) return null;
    const occurredAt = new Date(entry.time);
    if (Number.isNaN(occurredAt.getTime())) return null;
    const statusCode = Number(entry.status) || 0;
    const name = eventName({
      path: parsed.path,
      contentType: entry.content_type,
      statusCode,
    });
    if (!name) return null;
    const userAgent = String(entry.user_agent || "").slice(0, 1000) || null;
    const remoteAddress = String(entry.remote_addr || "unknown");
    if (!isMeaningfulVisitor({ userAgent, remoteAddress })) return null;
    return {
      eventKey: hash(`${entry.time}|${remoteAddress}|${entry.request}|${entry.status}|${entry.bytes_sent}`),
      eventName: name,
      path: parsed.path.slice(0, 512),
      method: parsed.method,
      statusCode,
      bytesSent: Number(entry.bytes_sent) || 0,
      referrer: sanitizeReferrer(entry.referer),
      userAgent,
      visitorKey: hash(`${visitorSalt}:${remoteAddress}:${userAgent || "unknown"}`).slice(0, 32),
      ipAddress: name === "page_view" || name.startsWith("download_") || name === "download" ? remoteAddress : null,
      occurredAt,
    };
  } catch {
    return null;
  }
}

export function daysFromUrl(url) {
  const value = Number(url.searchParams.get("days") || 30);
  return Math.min(Math.max(Number.isFinite(value) ? Math.trunc(value) : 30, 1), 365);
}
