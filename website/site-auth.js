const nav = document.querySelector(".site-nav");

function escapeText(value) {
  return String(value).replace(/[&<>"']/g, (character) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[character]);
}

function showSignedOut() {
  const login = nav.querySelector(".nav-signin");
  if (!login) return;
  login.textContent = "Log in";
  login.href = "/signin.html";
  login.classList.add("nav-login");
  if (!nav.querySelector(".nav-signup")) {
    const signup = document.createElement("a");
    signup.className = "nav-signup";
    signup.href = "/signin.html?mode=signup";
    signup.textContent = "Sign up";
    login.after(signup);
  }
}

function showSignedIn(email) {
  const login = nav.querySelector(".nav-signin");
  if (!login) return;
  nav.querySelector(".nav-signup")?.remove();
  const menu = document.createElement("details");
  menu.className = "account-menu";
  menu.innerHTML = `<summary class="account-trigger" aria-label="Open account menu"><span class="default-avatar" aria-hidden="true">IT</span></summary><div class="account-popover"><p class="account-email">${escapeText(email)}</p><a href="/account.html">Personal center</a><button type="button" data-account-logout>Sign out</button></div>`;
  login.replaceWith(menu);
  menu.querySelector("[data-account-logout]").addEventListener("click", async () => {
    const button = menu.querySelector("[data-account-logout]");
    button.disabled = true;
    await fetch("/auth/logout", { method: "POST", credentials: "same-origin" }).catch(() => {});
    location.reload();
  });
}

const localStaticPreview = location.hostname === "localhost" || location.hostname === "127.0.0.1";
if (nav && localStaticPreview) {
  showSignedOut();
} else if (nav) {
  fetch("/auth/me", { credentials: "same-origin" })
    .then((response) => response.ok ? response.json() : { authenticated: false })
    .then((body) => body.authenticated ? showSignedIn(body.email) : showSignedOut())
    .catch(() => showSignedOut());
}
