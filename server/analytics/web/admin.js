const element = (id) => document.getElementById(id);
async function api(path, options = {}) {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), 15000);
  try {
    const response = await fetch(`/admin/api${path}`, {
      credentials: "same-origin",
      headers: { "Content-Type": "application/json", ...(options.headers || {}) },
      ...options,
      signal: controller.signal,
    });
    const body = await response.json().catch((error) => {
      if (controller.signal.aborted) throw error;
      return {};
    });
    if (!response.ok) {
      const error = new Error(body.message || "The request failed. Try again.");
      error.status = response.status;
      throw error;
    }
    return body;
  } catch (error) {
    if (controller.signal.aborted) throw new Error("The request timed out. Check your connection and try again.");
    throw error;
  } finally {
    clearTimeout(timer);
  }
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

function formatStatus(value) {
  const code = Number(value || 0);
  if (code >= 200 && code < 300) {
    const successLabels = { 200: "200 OK", 201: "201 Created", 202: "202 Accepted", 206: "206 Partial content" };
    return successLabels[code] || `${code} Success`;
  }
  if (code >= 300 && code < 400) return `${code} Redirect`;
  const labels = {
    400: "400 Bad request",
    401: "401 Unauthorized",
    403: "403 Forbidden",
    404: "404 Not found",
    405: "405 Method not allowed",
    408: "408 Request timeout",
    429: "429 Too many requests",
    500: "500 Internal server error",
    502: "502 Bad gateway",
    503: "503 Service unavailable",
    504: "504 Gateway timeout",
  };
  return labels[code] || `${code} HTTP error`;
}

function empty(message) {
  return `<p class="empty">${escapeHtml(message)}</p>`;
}

function table(rows, columns) {
  if (!rows.length) return empty("No data for this period.");
  return `<table><thead><tr>${columns.map(([label]) => `<th>${escapeHtml(label)}</th>`).join("")}</tr></thead><tbody>${rows.map((row) => `<tr>${columns.map(([, key, formatter, className]) => `<td class="${className || ""}">${escapeHtml(formatter ? formatter(row[key], row) : row[key])}</td>`).join("")}</tr>`).join("")}</tbody></table>`;
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
  renderMetrics("downloads-metrics", [
    ["Download page visitors", overview.download_page_visitors],
    ["Successful requests", overview.downloads],
    ["Failed requests", overview.download_failures],
    ["No download after visit", Math.max(0, Number(overview.download_page_visitors || 0) - Number(overview.downloads || 0))],
  ]);
  renderMetrics("errors-metrics", [["Page errors", overview.errors]]);
  element("freshness").textContent = overview.latest_event_at ? `Latest data: ${formatTime(overview.latest_event_at)}` : "No traffic recorded yet";
}

// Render directly from report data; no external chart download can block display.
function renderLineChart(target, labels, datasets, ariaLabel) {
  const container = element(target);
  if (!labels.length || !datasets.length) {
    container.innerHTML = empty("No data for this period.");
    return;
  }
  const width = Math.max(300, container.clientWidth || 640);
  const height = 250, left = 48, right = width - 16, top = 14, bottom = 218;
  const rawMax = Math.max(1, ...datasets.flatMap((set) => set.values));
  const step = Math.max(1, Math.ceil(rawMax / 4));
  const max = step * 4;
  const x = (index) => left + (right - left) * (labels.length === 1 ? 0.5 : index / (labels.length - 1));
  const y = (value) => bottom - (bottom - top) * value / max;
  const grid = Array.from({ length: 5 }, (_, index) => {
    const value = index * step;
    return `<line x1="${left}" x2="${right}" y1="${y(value)}" y2="${y(value)}" stroke="#e8eeec"/><text x="${left - 8}" y="${y(value) + 4}" text-anchor="end">${formatNumber(value)}</text>`;
  }).join("");
  const tickCount = Math.min(labels.length, width < 450 ? 3 : 6);
  const tickIndices = new Set(Array.from({ length: tickCount }, (_, index) => Math.round(index * (labels.length - 1) / Math.max(1, tickCount - 1))));
  const ticks = labels.map((label, index) => tickIndices.has(index)
    ? `<text x="${x(index)}" y="240" text-anchor="middle">${escapeHtml(label)}</text>` : "").join("");
  const lines = datasets.map((set) => `<g class="chart-series" data-label="${escapeHtml(set.label)}"><polyline fill="none" stroke="${set.color}" stroke-width="2.5" points="${set.values.map((value, index) => `${x(index)},${y(value)}`).join(" ")}"/>${set.values.map((value, index) => `<circle cx="${x(index)}" cy="${y(value)}" r="3" fill="${set.color}"><title>${escapeHtml(labels[index])}: ${escapeHtml(set.label)} ${formatNumber(value)}</title></circle>`).join("")}</g>`).join("");
  container.innerHTML = `<svg role="img" aria-label="${escapeHtml(ariaLabel)}" viewBox="0 0 ${width} ${height}" font-size="12" font-family="system-ui" fill="#65716e"><title>${escapeHtml(ariaLabel)}</title>${grid}${ticks}${lines}</svg><div class="legend chart-legend">${datasets.map((set) => `<span><svg width="18" height="3" aria-hidden="true"><path d="M0 1.5H18" stroke="${set.color}" stroke-width="3"/></svg>${escapeHtml(set.label)}</span>`).join("")}</div><details><summary>View chart data</summary><div class="table-wrap">${table(labels.map((label, index) => ({ day: label, ...Object.fromEntries(datasets.map((set, id) => [id, set.values[index]])) })), [["Date", "day"], ...datasets.map((set, id) => [set.label, id, formatNumber, "numeric"])])}</div></details>`;
}

function renderTrend(target, rows, series, ariaLabel) {
  const palette = { pv: "#0f7664", uv: "#c17d11", downloads: "#2563a6", errors: "#b64141" };
  renderLineChart(target, rows.map((row) => String(row.day).slice(5)), series.map((item) => ({
    label: item.label, color: palette[item.className], values: rows.map((row) => Number(row[item.field] || 0)),
  })), ariaLabel);
}

function renderCountryLines(target, rows, countries, field, label) {
  const days = [...new Set(rows.map((row) => row.day))].sort();
  const byDay = new Map(days.map((day) => [day, new Map()]));
  rows.forEach((row) => byDay.get(row.day)?.set(row.country, Number(row[field] || 0)));
  const colors = ["#0f7664", "#c17d11", "#2563a6", "#b64141", "#6d4c9b", "#2f855a"];
  renderLineChart(target, days.map((day) => day.slice(5)), countries.map((country, index) => ({
    label: country, color: colors[index], values: days.map((day) => byDay.get(day)?.get(country) || 0),
  })), label);
}

function renderCountryTrend(target, rows) {
  const countries = [...new Set(rows.map((row) => row.country))].slice(0, 6);
  renderCountryLines(target, rows, countries, "unique_visitors", "Daily unique visitors by country");
}

function renderCountrySummary(target, rows) {
  element(target).innerHTML = table(rows, [
    ["Country", "country"],
    ["PV", "page_views", formatNumber, "numeric"],
    ["UV", "unique_visitors", formatNumber, "numeric"],
  ]);
}

function renderDownloadCountryTrend(target, rows, summary) {
  // Pick countries by their total downloads for the selected period. The daily
  // result is ordered by date, so taking its first rows can show unrelated
  // countries with zero downloads on the day being inspected.
  const countries = summary
    .filter((row) => Number(row.downloads || 0) > 0)
    .sort((left, right) => Number(right.downloads || 0) - Number(left.downloads || 0) || left.country.localeCompare(right.country))
    .slice(0, 6)
    .map((row) => row.country);
  renderCountryLines(target, rows, countries, "downloads", "Daily package downloads by country");
}

function renderDownloadCountrySummary(target, rows) {
  element(target).innerHTML = table(rows, [
    ["Country", "country"],
    ["Downloads", "downloads", formatNumber, "numeric"],
    ["Failures", "failures", formatNumber, "numeric"],
  ]);
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

function renderDownloadEvents(target, rows) {
  const names = {
    download_page_view: "Download page visit",
    download_success: "Download success",
    download_failure: "Download failed",
    download: "Download success",
  };
  element(target).innerHTML = table(rows, [
    ["Time", "occurred_at", formatTime],
    ["Outcome", "event_name", (value) => names[value] || value],
    ["Request", "path"],
    ["Status", "status_code", formatStatus],
    ["Bytes", "bytes_sent", formatNumber, "numeric"],
    ["Country", "country_name", (value, row) => value || row.country_code || "Unknown"],
    ["Source", "referrer", (value) => value && value !== "-" ? value : "Direct"],
    ["Browser", "user_agent", (value) => value || "Unknown"],
  ]);
}

function renderErrors(target, rows) {
  element(target).innerHTML = table(rows, [
    ["Time", "occurred_at", formatTime],
    ["Path", "path"],
    ["Error", "status_code", formatStatus],
    ["Method", "method"],
    ["Bytes", "bytes_sent", formatNumber, "numeric"],
    ["Source", "referrer", (value) => value && value !== "-" ? value : "Direct"],
    ["Browser", "user_agent", (value) => value || "Unknown"],
  ]);
}

let dashboardGeneration = 0;
async function loadDashboard() {
  const generation = ++dashboardGeneration;
  const selected = document.querySelector('[data-tab].is-active')?.dataset.tab || "overview";
  const error = element("dashboard-error");
  error.hidden = true;
  const days = element("days").value;
  const failures = [];
  function report(label, requestError) {
    if (generation !== dashboardGeneration) return;
    if (requestError.status === 401) { dashboardGeneration++; showLogin(); return; }
    failures.push(`${label}: ${requestError.message}`);
    error.textContent = `${failures.join(" ")} Other available data is shown. Use Refresh to retry.`;
    error.hidden = false;
  }
  async function section(path, label, targets, render, chart) {
    targets.forEach((id) => { element(id).innerHTML = empty(`Loading ${label.toLowerCase()}…`); });
    try {
      const data = await api(`${path}?days=${days}`);
      if (generation !== dashboardGeneration) return;
      render(data);
      if (chart) chart(data);
    } catch (requestError) {
      if (generation !== dashboardGeneration) return;
      targets.forEach((id) => { element(id).innerHTML = empty(`${label} could not be loaded. Use Refresh to retry.`); });
      report(label, requestError);
    }
  }
  const tasks = [];
  if (selected !== "feedback") {
    tasks.push(section("/overview", "Summary", ["overview-metrics", "traffic-metrics", "downloads-metrics", "errors-metrics"], renderOverviewMetrics));
  }
  const trends = {
    overview: ["overview-trend", [{ field: "page_views", className: "pv", label: "Page views" }, { field: "unique_visitors", className: "uv", label: "Unique visitors" }]],
    traffic: ["traffic-trend", [{ field: "page_views", className: "pv", label: "Page views" }, { field: "unique_visitors", className: "uv", label: "Unique visitors" }]],
    downloads: ["downloads-trend", [{ field: "downloads", className: "downloads", label: "Downloads" }]],
    errors: ["errors-trend", [{ field: "errors", className: "errors", label: "Errors" }]],
  };
  if (trends[selected]) {
    const [target, series] = trends[selected];
    tasks.push(section("/daily", "Daily trend", [target], () => {}, (rows) => renderTrend(target, rows, series, "Daily activity")));
  }
  if (selected === "overview") {
    tasks.push(section("/pages", "Top pages", ["overview-pages"], (rows) => renderPages("overview-pages", rows)));
    tasks.push(section("/events", "Recent events", ["overview-events"], (rows) => renderEvents("overview-events", rows)));
  } else if (selected === "traffic") {
    tasks.push(section("/countries", "Countries", ["country-summary", "country-trend"], (data) => renderCountrySummary("country-summary", data.summary), (data) => renderCountryTrend("country-trend", data.daily)));
  } else if (selected === "downloads") {
    tasks.push(section("/downloads", "Download activity", ["downloads-events", "downloads-funnel"], (data) => {
      renderDownloadEvents("downloads-events", data.events);
      element("downloads-funnel").innerHTML = [
        ["Download page visitors", data.summary.page_visitors],
        ["Successful download visitors", data.summary.successful_visitors],
        ["Failed requests", data.summary.failures],
        ["No download after visiting", data.summary.no_download_visitors],
      ].map(([label, value]) => `<div class="funnel-item"><span>${escapeHtml(label)}</span><strong>${formatNumber(value)}</strong></div>`).join("");
    }));
    tasks.push(section("/download-countries", "Download countries", ["download-country-summary", "download-country-trend"], (data) => renderDownloadCountrySummary("download-country-summary", data.summary), (data) => renderDownloadCountryTrend("download-country-trend", data.daily, data.summary)));
  } else if (selected === "errors") {
    tasks.push(section("/errors", "Failed requests", ["errors-events"], (rows) => renderErrors("errors-events", rows)));
  } else if (selected === "feedback") {
    tasks.push(import("/admin/feedback.js").then(() => window.loadFeedback()).catch((requestError) => report("Feedback", requestError)));
  }
  await Promise.allSettled(tasks);
  if (generation === dashboardGeneration && failures.length) {
    element("freshness").textContent = "Some sections could not be updated. Available data is shown below.";
  }
}

function showLogin() {
  dashboardGeneration++;
  element("session-check").hidden = true;
  element("dashboard").hidden = true;
  element("login").hidden = false;
  element("username").focus();
}

async function showDashboard() {
  element("session-check").hidden = true;
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
  button.textContent = "Signing in…";
  button.setAttribute("aria-busy", "true");
  try {
    await api("/login", { method: "POST", body: JSON.stringify({ username: element("username").value, password: element("password").value }) });
    element("password").value = "";
    await showDashboard();
  } catch (requestError) {
    error.textContent = requestError.message;
    element("password").focus();
  } finally {
    button.disabled = false;
    button.textContent = "Sign in";
    button.removeAttribute("aria-busy");
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
    void loadDashboard();
  });
});
element("logout").addEventListener("click", async () => {
  dashboardGeneration++;
  try {
    await api("/logout", { method: "POST" });
    showLogin();
  } catch (error) {
    element("dashboard-error").textContent = `Sign out failed: ${error.message} Try again.`;
    element("dashboard-error").hidden = false;
  }
});

async function restoreSession() {
  element("login").hidden = true;
  element("session-check").hidden = false;
  element("session-message").textContent = "Checking your existing session...";
  element("session-retry").hidden = true;
  try {
    const session = await api("/me");
    if (session.authenticated === true) await showDashboard();
    else if (session.authenticated === false) showLogin();
    else throw new Error("Invalid session response.");
  } catch (error) {
    if (error.status === 401) { showLogin(); return; }
    element("session-message").textContent = `Could not check your session. ${error.message} Your session has not been cleared.`;
    element("session-retry").hidden = false;
  }
}
element("session-retry").addEventListener("click", () => void restoreSession());

// Enable submission only after the JSON POST handler has been registered.
element("login-button").disabled = false;
element("login-button").textContent = "Sign in";
element("login-status").hidden = true;
void restoreSession();
