import assert from "node:assert/strict";
import test from "node:test";
import {
  daysFromUrl,
  eventName,
  isMeaningfulVisitor,
  parseLogLine,
  parseRequestLine,
  passwordDigest,
  safeEqual,
  sanitizeReferrer,
} from "./lib.mjs";

test("parses request paths without query strings", () => {
  assert.deepEqual(parseRequestLine("GET /downloads.html?source=nav HTTP/2.0"), {
    method: "GET",
    path: "/downloads.html",
  });
  assert.equal(parseRequestLine("not a request"), null);
});

test("removes query parameters and fragments from referrers", () => {
  assert.equal(
    sanitizeReferrer("https://example.com/download?token=secret&utm_source=test#section"),
    "https://example.com/download",
  );
  assert.equal(sanitizeReferrer("-"), null);
  assert.equal(sanitizeReferrer("not a URL"), null);
});

test("classifies only public page, download, and error events", () => {
  assert.equal(eventName({ path: "/", contentType: "text/html", statusCode: 200 }), "page_view");
  assert.equal(eventName({ path: "/downloads/file.zip", contentType: "application/zip", statusCode: 200 }), "download_success");
  assert.equal(eventName({ path: "/downloads/file.zip", contentType: "application/zip", statusCode: 404 }), "download_failure");
  assert.equal(eventName({ path: "/downloads.html", contentType: "text/html", statusCode: 200 }), "download_page_view");
  assert.equal(eventName({ path: "/wp-login.php", contentType: "text/html", statusCode: 200 }), null);
  assert.equal(eventName({ path: "/missing-probe", contentType: "text/html", statusCode: 404 }), null);
  assert.equal(eventName({ path: "/missing", contentType: "text/html", statusCode: 404 }), null);
  assert.equal(eventName({ path: "/privacy.html", contentType: "text/html", statusCode: 404 }), "page_error");
  assert.equal(eventName({ path: "/admin/", contentType: "text/html", statusCode: 200 }), null);
  assert.equal(eventName({ path: "/styles.css", contentType: "text/css", statusCode: 200 }), null);
});

test("hashes visitor identity while retaining IP only for normal page analytics", () => {
  const event = parseLogLine(JSON.stringify({
    time: "2026-09-23T12:00:00+00:00",
    remote_addr: "203.0.113.10",
    request: "GET / HTTP/2.0",
    status: 200,
    bytes_sent: 100,
    content_type: "text/html; charset=utf-8",
    referer: "-",
    user_agent: "Example Browser",
  }), "test-salt");
  assert.equal(event.eventName, "page_view");
  assert.equal(event.visitorKey.length, 32);
  assert.equal(event.ipAddress, "203.0.113.10");
});

test("clamps reporting windows", () => {
  assert.equal(daysFromUrl(new URL("https://itgla.com/admin/api/overview?days=0")), 1);
  assert.equal(daysFromUrl(new URL("https://itgla.com/admin/api/overview?days=900")), 365);
  assert.equal(daysFromUrl(new URL("https://itgla.com/admin/api/overview?days=invalid")), 30);
});

test("filters probe clients and non-public addresses", () => {
  assert.equal(isMeaningfulVisitor({ userAgent: "Mozilla/5.0", remoteAddress: "203.0.113.10" }), true);
  assert.equal(isMeaningfulVisitor({ userAgent: "curl/8.0", remoteAddress: "203.0.113.10" }), false);
  assert.equal(isMeaningfulVisitor({ userAgent: "Mozilla/5.0 (compatible; SemrushBot/7)", remoteAddress: "203.0.113.10" }), false);
  assert.equal(isMeaningfulVisitor({ userAgent: "Mozilla/5.0", remoteAddress: "192.168.1.4" }), false);
  assert.equal(isMeaningfulVisitor({ userAgent: "", remoteAddress: "203.0.113.10" }), false);
});

test("verifies scrypt password digests using constant-time comparison", () => {
  const digest = passwordDigest("correct horse battery staple", "0123456789abcdef");
  assert.equal(safeEqual(digest, passwordDigest("correct horse battery staple", "0123456789abcdef")), true);
  assert.equal(safeEqual(digest, passwordDigest("wrong", "0123456789abcdef")), false);
});
