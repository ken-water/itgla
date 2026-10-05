const form = document.getElementById("feedback-form");
const status = document.getElementById("feedback-status");

form?.addEventListener("submit", async (event) => {
  event.preventDefault();
  const button = form.querySelector("button[type=submit]");
  status.textContent = "Sending…";
  button.disabled = true;
  try {
    const response = await fetch("/api/feedback", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(Object.fromEntries(new FormData(form))),
    });
    const body = await response.json().catch(() => ({}));
    if (!response.ok) throw new Error(body.message || "The feedback could not be sent.");
    form.reset();
    status.textContent = "Thanks — your feedback was received.";
  } catch (error) {
    status.textContent = error.message;
  } finally {
    button.disabled = false;
  }
});
