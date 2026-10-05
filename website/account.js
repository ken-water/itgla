const accountStatus = document.getElementById("account-status");
fetch("/auth/me", { credentials: "same-origin" })
  .then((response) => response.json())
  .then((body) => {
    if (!body.authenticated) {
      location.replace("/signin.html");
      return;
    }
    accountStatus.textContent = body.email;
  })
  .catch(() => location.replace("/signin.html"));
