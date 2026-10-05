const emailForm = document.getElementById("email-signin-form");
const authStatus = document.getElementById("auth-status");
const pendingKey = "itgla_login_attempt";
let pollTimer;
function watchAttempt(requestId) {
  localStorage.setItem(pendingKey, requestId);
  clearInterval(pollTimer);
  pollTimer = setInterval(async () => {
    const response = await fetch(`/auth/email/status?request_id=${encodeURIComponent(requestId)}`, { credentials: "same-origin" });
    if (!response.ok) return;
    const body = await response.json();
    if (!body.authenticated) return;
    clearInterval(pollTimer);
    localStorage.removeItem(pendingKey);
    location.replace("/account.html");
  }, 2000);
}
const existingAttempt = localStorage.getItem(pendingKey);
if (existingAttempt) {
  authStatus.textContent = "Waiting for the verification link. You can open it on your phone; this browser will continue automatically.";
  watchAttempt(existingAttempt);
}
if (new URLSearchParams(location.search).get("verified") === "1") {
  authStatus.textContent = "You are signed in. Your verification link was used successfully.";
}

emailForm?.addEventListener("submit", async (event) => {
  event.preventDefault();
  const button = emailForm.querySelector("button");
  button.disabled = true;
  authStatus.textContent = "Sending verification email…";
  try {
    const response = await fetch("/auth/email/start", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ email: emailForm.email.value }),
    });
    const body = await response.json().catch(() => ({}));
    if (!response.ok) throw new Error(body.message || "Unable to send verification email.");
    authStatus.textContent = "Check your inbox on any device. When you open the link, this browser will sign in automatically.";
    if (body.request_id) watchAttempt(body.request_id);
    emailForm.reset();
  } catch (error) {
    authStatus.textContent = error.message;
  } finally {
    button.disabled = false;
  }
});
