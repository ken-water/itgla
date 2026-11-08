import crypto from "node:crypto";

const ttl = 10 * 60 * 1000;
export function createGoogleAuth({ clientId, clientSecret, publicBaseUrl, fetchImpl = fetch, now = Date.now }) {
  const pending = new Map();
  const redirectUri = new URL("/auth/google/callback", publicBaseUrl).href;
  const enabled = Boolean(clientId && clientSecret);
  const cookie = (value, age) => `__Host-itgla_google_state=${value}; Path=/; Max-Age=${age}; HttpOnly; Secure; SameSite=Lax`;
  return {
    enabled,
    clearCookie: cookie("", 0),
    start() {
      if (!enabled) throw new Error("Google sign-in is not configured.");
      for (const [key, item] of pending) if (item.expires <= now()) pending.delete(key);
      if (pending.size >= 1000) throw new Error("Please try again shortly.");
      const state = crypto.randomBytes(32).toString("hex");
      const verifier = crypto.randomBytes(32).toString("base64url");
      pending.set(state, { verifier, expires: now() + ttl });
      const url = new URL("https://accounts.google.com/o/oauth2/v2/auth");
      url.search = new URLSearchParams({ client_id: clientId, redirect_uri: redirectUri, response_type: "code", scope: "openid email", state, code_challenge: crypto.createHash("sha256").update(verifier).digest("base64url"), code_challenge_method: "S256", prompt: "select_account" }).toString();
      return { location: url.href, cookie: cookie(state, 600) };
    },
    async finish({ state, cookieState, code, error }) {
      if (!enabled || !/^[a-f0-9]{64}$/.test(state || "") || state !== cookieState) throw new Error("Invalid sign-in state.");
      const attempt = pending.get(state);
      pending.delete(state); // Every valid browser-bound callback consumes its attempt, even on failure.
      if (!attempt || attempt.expires <= now() || error || typeof code !== "string" || !code || code.length > 4096) throw new Error("Sign-in expired or cancelled.");
      const tokens = await fetchImpl("https://oauth2.googleapis.com/token", {
        method: "POST", headers: { "Content-Type": "application/x-www-form-urlencoded" },
        body: new URLSearchParams({ client_id: clientId, client_secret: clientSecret, code, code_verifier: attempt.verifier, grant_type: "authorization_code", redirect_uri: redirectUri }),
        signal: AbortSignal.timeout(8000), redirect: "error",
      });
      if (!tokens.ok) throw new Error("Google sign-in failed.");
      const token = await tokens.json();
      if (typeof token.access_token !== "string" || token.token_type?.toLowerCase() !== "bearer") throw new Error("Invalid Google token.");
      const response = await fetchImpl("https://openidconnect.googleapis.com/v1/userinfo", {
        headers: { Authorization: `Bearer ${token.access_token}` }, signal: AbortSignal.timeout(8000), redirect: "error",
      });
      if (!response.ok) throw new Error("Google identity unavailable.");
      const user = await response.json();
      if (typeof user.sub !== "string" || !user.sub || user.sub.length > 255 || user.email_verified !== true || typeof user.email !== "string" || user.email.length > 254 || !/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(user.email)) throw new Error("Google email is not verified.");
      // Google is authoritative for Gmail and verified Workspace domains. Other email
      // addresses must use our email link rather than implicitly linking an existing account.
      if (!user.email.toLowerCase().endsWith("@gmail.com") && (typeof user.hd !== "string" || !user.hd)) throw new Error("Use email sign-in for this address.");
      return { subject: user.sub, email: user.email.toLowerCase() };
    },
  };
}
