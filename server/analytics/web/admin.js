const element = (id) => document.getElementById(id);

async function api(path, options = {}) {
  const response = await fetch(`/admin/api${path}`, {
    credentials: "same-origin",
    headers: { "Content-Type": "application/json", ...(options.headers || {}) },
    ...options,
  });
  const body = await response.json().catch(() => ({}));
  if (!response.ok) {
    const error = new Error(body.message || "The request failed. Try again.");
    error.status = response.status;
    throw error;
  }
  return body;
}

function escapeHtml(value) {
  return String(value ?? "").replace(/[&<>"']/g, (character) => ({
    "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;",
  })[character]);
}

function formatNumber(value) {
  return Number(value || 0).toLocaleString("en-US");
}

function formatTime(value) {
  return value ? new Date(value).toLocaleString("en-US", { hour12: false }) : "None";
}

function empty(message) {
  return `<p class="empty">${escapeHtml(message)}</p>`;
}

function table(rows, columns) {
  if (!rows.length) return empty("No data for this period.");
  return `<table><thead><tr>${columns.map(([label]) => `<th>${escapeHtml(label)}</th>`).join("")}</tr></thead><tbody>${rows.map((row) => `<tr>${columns.map(([, key, formatter, className]) => `<td class="${className || ""}">${escapeHtml(formatter ? formatter(row[key]) : row[key])}</td>`).join("")}</tr>`).join("")}</tbody></table>`;
}

function renderMetrics(target, items) {
  element(target).innerHTML = items.map(([label, value]) => `<article class="metric"><span>${label}</span><strong>${formatNumber(value)}</strong></article>`).join("");
}

function renderOverviewMetrics(overview) {
  renderMetrics("overview-metrics", [
    ["Page views (PV)", overview.page_views],
    ["Unique visitors (UV)", overview.unique_visitors],
    ["Package downloads", overview.downloads],
    ["Page errors", overview.errors],
  ]);
  renderMetrics("traffic-metrics", [["Page views (PV)", overview.page_views], ["Unique visitors (UV)", overview.unique_visitors]]);
  renderMetrics("downloads-metrics", [["Package downloads", overview.downloads]]);
  renderMetrics("errors-metrics", [["Page errors", overview.errors]]);
  element("freshness").textContent = overview.latest_event_at ? `Latest data: ${formatTime(overview.latest_event_at)}` : "No traffic recorded yet";
}

function renderTrend(target, rows, series, ariaLabel) {
  const container = element(target);
  if (!rows.length) {
    container.innerHTML = empty("No data for this period.");
    return;
  }
  const width = 760;
  const height = 270;
  const padding = { top: 18, right: 16, bottom: 34, left: 38 };
  const maximum = Math.max(1, ...rows.flatMap((row) => series.map((item) => Number(row[item.field]))));
  const x = (index) => padding.left + (index * (width - padding.left - padding.right)) / Math.max(1, rows.length - 1);
  const y = (value) => height - padding.bottom - (Number(value) * (height - padding.top - padding.bottom)) / maximum;
  const grids = [0, .5, 1].map((ratio) => `<line class="chart-grid" x1="${padding.left}" x2="${width - padding.right}" y1="${y(maximum * ratio)}" y2="${y(maximum * ratio)}"></line><text class="chart-label" x="2" y="${y(maximum * ratio) + 4}">${Math.round(maximum * ratio)}</text>`).join("");
  const labels = rows.map((row, index) => {
    const interval = Math.max(1, Math.ceil(rows.length / 6));
    if (index % interval !== 0 && index !== rows.length - 1) return "";
    return `<text class="chart-label" text-anchor="middle" x="${x(index)}" y="${height - 7}">${escapeHtml(String(row.day).slice(5))}</text>`;
  }).join("");
  const lines = series.map((item) => `<polyline class="chart-${item.className}" points="${rows.map((row, index) => `${x(index)},${y(row[item.field])}`).join(" ")}"></polyline>`).join("");
  container.innerHTML = `<svg viewBox="0 0 ${width} ${height}" role="img" aria-label="${escapeHtml(ariaLabel)}">${grids}${lines}${labels}</svg>`;
}

function renderPages(target, rows) {
  element(target).innerHTML = table(rows, [
    ["Page", "path"], ["PV", "page_views", formatNumber, "numeric"], ["UV", "unique_visitors", formatNumber, "numeric"],
  ]);
}

function renderEvents(target, rows) {
  const names = { page_view: "Page view", download: "Download", page_error: "Page error" };
  element(target).innerHTML = table(rows, [
    ["Time", "occurred_at", formatTime], ["Type", "event_name", (value) => names[value] || value], ["Path", "path"], ["Status", "status_code", formatNumber, "numeric"], ["Source", "referrer", (value) => value && value !== "-" ? value : "Direct"],
  ]);
}

async function loadDashboard() {
  const error = element("dashboard-error");
  error.hidden = true;
  const days = element("days").value;
  try {
    const [overview, daily, pages, events] = await Promise.all([
      api(`/overview?days=${days}`), api(`/daily?days=${days}`), api(`/pages?days=${days}`), api(`/events?days=${days}`),
    ]);
    renderOverviewMetrics(overview);
    renderTrend("overview-trend", daily, [{ field: "page_views", className: "pv" }, { field: "unique_visitors", className: "uv" }], "Daily page views and unique visitors");
    renderTrend("traffic-trend", daily, [{ field: "page_views", className: "pv" }, { field: "unique_visitors", className: "uv" }], "Daily page views and unique visitors");
    renderTrend("downloads-trend", daily, [{ field: "downloads", className: "downloads" }], "Daily package downloads");
    renderTrend("errors-trend", daily, [{ field: "errors", className: "errors" }], "Daily page errors");
    renderPages("overview-pages", pages);
    renderEvents("overview-events", events);
    renderEvents("downloads-events", events.filter((event) => event.event_name === "download"));
    renderEvents("errors-events", events.filter((event) => event.event_name === "page_error"));
  } catch (requestError) {
    if (requestError.status === 401) return showLogin();
    error.textContent = requestError.message;
    error.hidden = false;
  }
}

function showLogin() {
  element("dashboard").hidden = true;
  element("login").hidden = false;
  element("username").focus();
}

async function showDashboard() {
  element("login").hidden = true;
  element("dashboard").hidden = false;
  await loadDashboard();
}

element("login-form").addEventListener("submit", async (event) => {
  event.preventDefault();
  const button = element("login-button");
  const error = element("login-error");
  error.textContent = "";
  button.disabled = true;
  try {
    await api("/login", { method: "POST", body: JSON.stringify({ username: element("username").value, password: element("password").value }) });
    element("password").value = "";
    await showDashboard();
  } catch (requestError) {
    error.textContent = requestError.message;
    element("password").focus();
  } finally {
    button.disabled = false;
  }
});

element("days").addEventListener("change", () => void loadDashboard());
element("refresh").addEventListener("click", () => void loadDashboard());
document.querySelectorAll("[data-tab]").forEach((tab) => {
  tab.addEventListener("click", () => {
    const selected = tab.dataset.tab;
    document.querySelectorAll("[data-tab]").forEach((item) => {
      const active = item === tab;
      item.classList.toggle("is-active", active);
      item.setAttribute("aria-selected", String(active));
    });
    document.querySelectorAll("[data-panel]").forEach((panel) => {
      const active = panel.dataset.panel === selected;
      panel.classList.toggle("is-active", active);
      panel.hidden = !active;
    });
  });
});
element("logout").addEventListener("click", async () => {
  await api("/logout", { method: "POST" }).catch(() => {});
  showLogin();
});

api("/me").then((session) => session.authenticated ? showDashboard() : showLogin()).catch(showLogin);
