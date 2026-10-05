const feedbackTarget = document.getElementById("feedback-list");
const feedbackStatus = document.getElementById("feedback-status");

function feedbackEscape(value) {
  return String(value ?? "").replace(/[&<>"']/g, (character) => ({
    "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;",
  })[character]);
}

async function loadFeedback() {
  const response = await fetch("/admin/api/feedback", { credentials: "same-origin" });
  if (!response.ok) throw new Error("Feedback could not be loaded.");
  const rows = await response.json();
  feedbackTarget.innerHTML = rows.length
    ? rows.map((row) => `<article class="feedback-entry ${row.read_at ? "is-read" : ""}" data-id="${row.id}">
      <div class="feedback-entry-head"><strong>#${row.id}</strong><time>${new Date(row.created_at).toLocaleString("en-US", { hour12: false })}</time><span>${feedbackEscape(row.email_status)}</span></div>
      <p>${feedbackEscape(row.message)}</p>
      <div class="feedback-entry-foot">${row.reply_email ? `<a href="mailto:${feedbackEscape(row.reply_email)}">${feedbackEscape(row.reply_email)}</a>` : "<span>No reply email</span>"}${row.read_at ? "<span>Read</span>" : `<button type="button" data-read="${row.id}">Mark read</button>`}</div>
    </article>`).join("")
    : "<p class=\"empty\">No feedback has been submitted yet.</p>";
  feedbackStatus.textContent = `${rows.length} message${rows.length === 1 ? "" : "s"}`;
  feedbackTarget.querySelectorAll("[data-read]").forEach((button) => {
    button.addEventListener("click", async () => {
      await fetch(`/admin/api/feedback/${button.dataset.read}/read`, { method: "POST", credentials: "same-origin" });
      await loadFeedback();
    });
  });
}

window.loadFeedback = loadFeedback;
