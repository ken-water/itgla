const element = (id) => document.getElementById(id);
const charts = new Map();

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
  charts.get(target)?.destroy();
  if (!rows.length) {
    container.innerHTML = empty("No data for this period.");
    return;
  }
  container.innerHTML = '<canvas role="img"></canvas>';
  const canvas = container.querySelector("canvas");
  canvas.setAttribute("aria-label", ariaLabel);
  const palette = {
    pv: { border: "#0f7664", background: "rgba(15, 118, 100, .12)" },
    uv: { border: "#c17d11", background: "rgba(193, 125, 17, .12)" },
    downloads: { border: "#2563a6", background: "rgba(37, 99, 166, .12)" },
    errors: { border: "#b64141", background: "rgba(182, 65, 65, .12)" },
  };
  const chart = new Chart(canvas, {
    type: "line",
    data: {
      labels: rows.map((row) => String(row.day).slice(5)),
      datasets: series.map((item) => {
        const colors = palette[item.className];
        return {
          label: item.label,
          data: rows.map((row) => Number(row[item.field] || 0)),
          borderColor: colors.border,
          backgroundColor: colors.background,
          borderWidth: 2.5,
          pointRadius: rows.length > 45 ? 0 : 3,
          pointHoverRadius: 5,
          pointBackgroundColor: colors.border,
          tension: 0.35,
          fill: series.length === 1,
        };
      }),
    },
    options: {
      responsive: true,
      maintainAspectRatio: false,
      interaction: { mode: "index", intersect: false },
      plugins: {
        legend: { display: false },
        tooltip: {
          backgroundColor: "#173f36",
          padding: 12,
          displayColors: true,
          callbacks: { title: (items) => `Date: ${items[0].label}` },
        },
      },
      scales: {
        x: {
          grid: { display: false },
          ticks: { color: "#65716e", maxTicksLimit: 7, maxRotation: 0 },
        },
        y: {
          beginAtZero: true,
          suggestedMax: 1,
          grid: { color: "#e8eeec" },
          ticks: { color: "#65716e", precision: 0, padding: 8 },
        },
      },
    },
  });
  charts.set(target, chart);
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
    renderTrend("overview-trend", daily, [{ field: "page_views", className: "pv", label: "Page views" }, { field: "unique_visitors", className: "uv", label: "Unique visitors" }], "Daily page views and unique visitors");
    renderTrend("traffic-trend", daily, [{ field: "page_views", className: "pv", label: "Page views" }, { field: "unique_visitors", className: "uv", label: "Unique visitors" }], "Daily page views and unique visitors");
    renderTrend("downloads-trend", daily, [{ field: "downloads", className: "downloads", label: "Downloads" }], "Daily package downloads");
    renderTrend("errors-trend", daily, [{ field: "errors", className: "errors", label: "Errors" }], "Daily page errors");
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
