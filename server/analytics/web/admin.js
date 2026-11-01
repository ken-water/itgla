const element = (id) => document.getElementById(id);
const charts = new Map();

let chartLoading;
let chartAttempt = 0;
function loadCharts() {
  if (typeof window.Chart === "function") return Promise.resolve(true);
  if (!chartLoading) {
    // Keep observing slow transfers so late success can still draw charts.
    // A distinct URL on retry avoids the browser's cached failed module result.
    const url = "/admin/chart.umd.min.js" + (chartAttempt++ ? `?retry=${chartAttempt}` : "");
    chartLoading = import(url).then(() => typeof window.Chart === "function", () => false);
  }
  return chartLoading.then((loaded) => {
    if (!loaded) chartLoading = undefined;
    return loaded;
  });
}

function chartUnavailable(container) {
  if (typeof window.Chart === "function") return false;
  container.innerHTML = empty("Chart unavailable. Your data is still available in the tables. Refresh to retry.");
  return true;
}

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

function renderTrend(target, rows, series, ariaLabel) {
  const container = element(target);
  if (chartUnavailable(container)) return;
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

function renderCountryTrend(target, rows) {
  if (chartUnavailable(element(target))) return;
  const countries = [...new Set(rows.map((row) => row.country))].slice(0, 6);
  const days = [...new Set(rows.map((row) => row.day))];
  const byDay = new Map(days.map((day) => [day, new Map()]));
  rows.forEach((row) => byDay.get(row.day)?.set(row.country, Number(row.unique_visitors || 0)));
  const colors = ["#0f7664", "#c17d11", "#2563a6", "#b64141", "#6d4c9b", "#2f855a"];
  const container = element(target);
  charts.get(target)?.destroy();
  if (!countries.length) {
    container.innerHTML = empty("No country data for this period.");
    return;
  }
  container.innerHTML = '<canvas role="img" aria-label="Daily unique visitors by country"></canvas>';
  const canvas = container.querySelector("canvas");
  charts.set(target, new Chart(canvas, {
    type: "line",
    data: {
      labels: days.map((day) => day.slice(5)),
      datasets: countries.map((country, index) => ({
        label: country,
        data: days.map((day) => byDay.get(day)?.get(country) || 0),
        borderColor: colors[index % colors.length],
        backgroundColor: "transparent",
        borderWidth: 2,
        pointRadius: days.length > 45 ? 0 : 2,
        tension: 0.3,
      })),
    },
    options: {
      responsive: true,
      maintainAspectRatio: false,
      interaction: { mode: "index", intersect: false },
      plugins: {
        legend: { display: true, position: "bottom", labels: { color: "#65716e", boxWidth: 18, usePointStyle: true } },
        tooltip: { backgroundColor: "#173f36", padding: 12, callbacks: { title: (items) => `Date: ${items[0].label}` } },
      },
      scales: {
        x: { grid: { display: false }, ticks: { color: "#65716e", maxTicksLimit: 7, maxRotation: 0 } },
        y: { beginAtZero: true, suggestedMax: 1, grid: { color: "#e8eeec" }, ticks: { color: "#65716e", precision: 0, padding: 8 } },
      },
    },
  }));
}

function renderCountrySummary(target, rows) {
  element(target).innerHTML = table(rows, [
    ["Country", "country"],
    ["PV", "page_views", formatNumber, "numeric"],
    ["UV", "unique_visitors", formatNumber, "numeric"],
  ]);
}

function renderDownloadCountryTrend(target, rows, summary) {
  if (chartUnavailable(element(target))) return;
  // Pick countries by their total downloads for the selected period. The daily
  // result is ordered by date, so taking its first rows can show unrelated
  // countries with zero downloads on the day being inspected.
  const countries = summary
    .filter((row) => Number(row.downloads || 0) > 0)
    .sort((left, right) => Number(right.downloads || 0) - Number(left.downloads || 0) || left.country.localeCompare(right.country))
    .slice(0, 6)
    .map((row) => row.country);
  const days = [...new Set(rows.map((row) => row.day))];
  const byDay = new Map(days.map((day) => [day, new Map()]));
  rows.forEach((row) => byDay.get(row.day)?.set(row.country, Number(row.downloads || 0)));
  const colors = ["#2563a6", "#0f7664", "#c17d11", "#b64141", "#6d4c9b", "#2f855a"];
  const container = element(target);
  charts.get(target)?.destroy();
  if (!countries.length) {
    container.innerHTML = empty("No download country data for this period.");
    return;
  }
  container.innerHTML = '<canvas role="img" aria-label="Daily package downloads by country"></canvas>';
  charts.set(target, new Chart(container.querySelector("canvas"), {
    type: "line",
    data: {
      labels: days.map((day) => day.slice(5)),
      datasets: countries.map((country, index) => ({
        label: country,
        data: days.map((day) => byDay.get(day)?.get(country) || 0),
        borderColor: colors[index % colors.length],
        backgroundColor: "transparent",
        borderWidth: 2,
        pointRadius: days.length > 45 ? 0 : 2,
        tension: 0.3,
      })),
    },
    options: {
      responsive: true,
      maintainAspectRatio: false,
      interaction: { mode: "index", intersect: false },
      plugins: {
        legend: { display: true, position: "bottom", labels: { color: "#65716e", boxWidth: 18, usePointStyle: true } },
        tooltip: { backgroundColor: "#173f36", padding: 12, callbacks: { title: (items) => `Date: ${items[0].label}` } },
      },
      scales: {
        x: { grid: { display: false }, ticks: { color: "#65716e", maxTicksLimit: 7, maxRotation: 0 } },
        y: { beginAtZero: true, suggestedMax: 1, grid: { color: "#e8eeec" }, ticks: { color: "#65716e", precision: 0, padding: 8 } },
      },
    },
  }));
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
      if (chart) {
        const timer = setTimeout(() => {
          if (generation !== dashboardGeneration) return;
          targets.filter((id) => element(id).classList.contains("chart")).forEach((id) => {
            element(id).innerHTML = empty("Charts are still loading. Tables are ready; charts will appear automatically.");
          });
        }, 10000);
        void loadCharts().then(() => {
          clearTimeout(timer);
          if (generation === dashboardGeneration) chart(data);
        });
      }
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
  await api("/logout", { method: "POST" }).catch(() => {});
  showLogin();
});

// Enable submission only after the JSON POST handler has been registered.
element("login-button").disabled = false;
element("login-button").textContent = "Sign in";
element("login-status").hidden = true;
void import("/admin/feedback.js").catch(() => {});
api("/me").then((session) => session.authenticated ? showDashboard() : showLogin()).catch(showLogin);
