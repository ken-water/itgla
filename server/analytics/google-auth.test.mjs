import assert from "node:assert/strict";
import test from "node:test";
import { createGoogleAuth } from "./google-auth.mjs";

function fixture(user = { sub: "google-subject", email: "User@gmail.com", email_verified: true }) {
  const calls = [];
  const auth = createGoogleAuth({ clientId: "test-client", clientSecret: "test-secret", publicBaseUrl: "https://itgla.com", fetchImpl: async (url, options) => {
    calls.push({ url, options });
    return { ok: true, json: async () => url.endsWith("/token") ? { access_token: "test-access", token_type: "Bearer" } : user };
  } });
  const start = auth.start();
  const state = new URL(start.location).searchParams.get("state");
  return { auth, start, state, calls };
}

test("Google authorization uses fixed callback, PKCE and a secure browser-bound state cookie", async () => {
  const f = fixture();
  const url = new URL(f.start.location);
  assert.equal(url.origin, "https://accounts.google.com");
  assert.equal(url.searchParams.get("redirect_uri"), "https://itgla.com/auth/google/callback");
  assert.equal(url.searchParams.get("code_challenge_method"), "S256");
  assert.match(f.start.cookie, /HttpOnly; Secure; SameSite=Lax/);
  assert.equal(url.searchParams.get("client_secret"), null);
  const identity = await f.auth.finish({ state: f.state, cookieState: f.state, code: "test-code" });
  assert.deepEqual(identity, { subject: "google-subject", email: "user@gmail.com" });
  assert.ok(f.calls[0].options.body.get("code_verifier"));
  assert.equal(f.calls[1].options.headers.Authorization, "Bearer test-access");
  await assert.rejects(f.auth.finish({ state: f.state, cookieState: f.state, code: "replay" }));
});

test("Google callback rejects cross-browser state, cancelled consent and expired state before exchanging tokens", async () => {
  const f = fixture();
  await assert.rejects(f.auth.finish({ state: f.state, cookieState: "different", code: "code" }));
  assert.equal(f.calls.length, 0);
  await assert.rejects(f.auth.finish({ state: f.state, cookieState: f.state, error: "access_denied" }));
  await assert.rejects(f.auth.finish({ state: f.state, cookieState: f.state, code: "code" }));
  assert.equal(f.calls.length, 0);
  let time = 0;
  const auth = createGoogleAuth({ clientId: "id", clientSecret: "secret", publicBaseUrl: "https://itgla.com", now: () => time });
  const state = new URL(auth.start().location).searchParams.get("state");
  time = 600001;
  await assert.rejects(auth.finish({ state, cookieState: state, code: "code" }));
});

test("Google cannot sign in with unverified or non-authoritative email identities", async () => {
  for (const user of [
    { sub: "s", email: "user@gmail.com", email_verified: false },
    { sub: "s", email: "user@example.com", email_verified: true },
    { email: "user@gmail.com", email_verified: true },
  ]) {
    const f = fixture(user);
    await assert.rejects(f.auth.finish({ state: f.state, cookieState: f.state, code: "code" }));
  }
});

test("Google provider failures do not create an identity", async () => {
  const auth = createGoogleAuth({ clientId: "id", clientSecret: "secret", publicBaseUrl: "https://itgla.com", fetchImpl: async () => ({ ok: false }) });
  const state = new URL(auth.start().location).searchParams.get("state");
  await assert.rejects(auth.finish({ state, cookieState: state, code: "code" }));
  assert.throws(() => createGoogleAuth({ publicBaseUrl: "https://itgla.com" }).start());
});
