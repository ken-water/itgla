const emailForm = document.getElementById("email-signin-form");
const authStatus = document.getElementById("auth-status");
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
    authStatus.textContent = body.message || "Check your inbox for the verification email.";
    emailForm.reset();
  } catch (error) {
    authStatus.textContent = error.message;
  } finally {
    button.disabled = false;
  }
});
