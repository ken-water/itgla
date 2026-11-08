# Google website sign-in

Google sign-in supplements the website's email links. It does not sign in the desktop application or synchronize its inventory. The local preview is not a live OAuth deployment.

Set `ITGLA_GOOGLE_CLIENT_ID` and `ITGLA_GOOGLE_CLIENT_SECRET` only in the production service's protected environment file. Never place the client secret in HTML, Git, command output or browser storage. Neither variable is required for existing email sign-in; without both, the Google start endpoint returns 503.

Configure the Google Cloud web application with the exact authorized redirect URI `https://itgla.com/auth/google/callback`. The application needs only `openid email`. Configure the consent screen and permitted test users before acceptance. The client ID is public; the client secret remains server-side. A credential shared through chat should be rotated before production activation.

The server uses authorization code exchange with PKCE and a ten-minute, one-use state bound to a Secure, HttpOnly, SameSite=Lax cookie. It retrieves identity from Google's authenticated userinfo endpoint; it does not trust browser-supplied email or decode an unverified ID token. Verified Gmail/Google Workspace identities can sign in. Other email domains use the existing email link to avoid implicit linking with a non-authoritative Google email. Stable Google subjects map to existing website accounts, and authentication creates a hashed, thirty-day server session. Access tokens are not persisted. Callback queries are not logged by the new Nginx route.

Before production deployment, back up affected service/config files, apply the additive `google_identities` table, install the module and routes, validate Nginx, and test enabled/disabled configuration, successful Google consent, cancellation, expired/replayed state, email-link compatibility, session reload/logout and database rollback on failed session creation. Real Google consent requires the user's Google account and configured Cloud client; mocked tests do not establish provider acceptance.
