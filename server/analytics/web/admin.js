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
    if (controller.signal.aborted) throw new Error("The request timed out after 15 seconds. Try again.");
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
  if (!value) return "None";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return "None";
  const pad = (number) => String(number).padStart(2, "0");
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())} ${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}`;
}

// Feedback uses the same local-time display contract as all report tables.
window.formatAdminTime = formatTime;

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
  renderMetrics("download-request-metrics", [
    ["Download page views", overview.download_page_views],
    ["Successful download requests", overview.downloads],
    ["Failed download requests", overview.download_failures],
    ["Total package requests", Number(overview.downloads || 0) + Number(overview.download_failures || 0)],
  ]);
  renderMetrics("errors-metrics", [["Page errors", overview.errors]]);
  element("freshness").textContent = overview.latest_event_at ? `Latest data: ${formatTime(overview.latest_event_at)}` : "No traffic recorded yet";
}

// Monotone cubic curves keep each segment inside its data range.
function smoothPath(points) {
  if (points.length < 2) return `M${points[0].join(",")}`;
  const slopes = points.slice(1).map((point, index) => (point[1] - points[index][1]) / (point[0] - points[index][0]));
  const tangents = points.map((_, index) => {
    if (!index) return slopes[0];
    if (index === points.length - 1) return slopes[index - 1];
    const before = slopes[index - 1], after = slopes[index];
    return before * after <= 0 ? 0 : 2 * before * after / (before + after);
  });
  return `M${points[0].join(",")}` + points.slice(1).map((point, index) => {
    const previous = points[index], third = (point[0] - previous[0]) / 3;
    return `C${previous[0] + third},${previous[1] + third * tangents[index]} ${point[0] - third},${point[1] - third * tangents[index + 1]} ${point.join(",")}`;
  }).join("");
}

// Render directly from report data; no external chart download can block display.
function renderLineChart(target, labels, datasets, ariaLabel) {
  const container = element(target);
  if (!labels.length || !datasets.length) {
    container.innerHTML = empty("No data for this period.");
    return;
  }
  const width = Math.max(300, container.clientWidth || 640);
  const height = 250, left = 48, right = width - 42, top = 14, bottom = 218;
  const rawMax = Math.max(1, ...datasets.flatMap((set) => set.values));
  const step = Math.max(1, Math.ceil(rawMax / 4));
  const max = step * 4;
  const x = (index) => left + (right - left) * (labels.length === 1 ? 0.5 : index / (labels.length - 1));
  const y = (value) => bottom - (bottom - top) * value / max;
  const grid = Array.from({ length: 5 }, (_, index) => {
    const value = index * step;
    return `<line x1="${left}" x2="${right}" y1="${y(value)}" y2="${y(value)}" stroke="#e8eeec"/><text x="${left - 8}" y="${y(value) + 4}" text-anchor="end">${formatNumber(value)}</text>`;
  }).join("");
  const tickCount = Math.min(labels.length, width < 450 ? 2 : Math.min(6, Math.floor(width / 100)));
  const tickIndices = new Set(Array.from({ length: tickCount }, (_, index) => Math.round(index * (labels.length - 1) / Math.max(1, tickCount - 1))));
  const ticks = labels.map((label, index) => tickIndices.has(index)
    ? `<text x="${x(index)}" y="240" text-anchor="middle">${escapeHtml(label)}</text>` : "").join("");
  const lines = datasets.map((set) => {
    const points = set.values.map((value, index) => [x(index), y(value)]);
    const path = smoothPath(points);
    return `<g class="chart-series" data-label="${escapeHtml(set.label)}">${datasets.length === 1 ? `<path d="${path}L${x(labels.length - 1)},${bottom}L${x(0)},${bottom}Z" fill="${set.color}" opacity=".07"/>` : ""}<path d="${path}" fill="none" stroke="${set.color}" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"/>${points.map((point) => `<circle cx="${point[0]}" cy="${point[1]}" r="${labels.length > 45 ? 0 : 2.5}" fill="${set.color}"/>`).join("")}</g>`;
  }).join("");
  container.innerHTML = `<svg role="img" tabindex="0" aria-label="${escapeHtml(ariaLabel)}. Use Left and Right arrow keys to inspect dates." viewBox="0 0 ${width} ${height}" font-size="12" font-family="system-ui" fill="#65716e"><title>${escapeHtml(ariaLabel)}</title>${grid}${ticks}${lines}<g class="chart-cursor" visibility="hidden" pointer-events="none"></g><g class="chart-tooltip" role="tooltip" visibility="hidden" pointer-events="none"></g></svg><div class="legend chart-legend">${datasets.map((set) => `<span><svg width="18" height="3" aria-hidden="true"><path d="M0 1.5H18" stroke="${set.color}" stroke-width="3"/></svg>${escapeHtml(set.label)}</span>`).join("")}</div><details><summary>View chart data</summary><div class="table-wrap">${table(labels.map((label, index) => ({ day: label, ...Object.fromEntries(datasets.map((set, id) => [id, set.values[index]])) })), [["Date", "day"], ...datasets.map((set, id) => [set.label, id, formatNumber, "numeric"])])}</div></details>`;
  const svg = container.querySelector(":scope > svg");
  const cursor = svg.querySelector(".chart-cursor"), tooltip = svg.querySelector(".chart-tooltip");
  let selected = -1;
  function inspect(index) {
    index = Math.max(0, Math.min(labels.length - 1, index));
    if (index === selected && tooltip.getAttribute("visibility") === "visible") return;
    selected = index;
    const boxWidth = Math.min(width - 16, Math.max(200, ...datasets.map((set) => (set.label.length + formatNumber(set.values[index]).length) * 7.5 + 44)));
    const boxHeight = 42 + datasets.length * 23;
    const boxX = Math.max(8, Math.min(width - boxWidth - 8, x(index) + 14));
    cursor.innerHTML = `<line x1="${x(index)}" x2="${x(index)}" y1="${top}" y2="${bottom}" stroke="#91a9a2" stroke-dasharray="4 4"/>${datasets.map((set) => `<circle cx="${x(index)}" cy="${y(set.values[index])}" r="4.5" fill="${set.color}" stroke="white" stroke-width="2"/>`).join("")}`;
    tooltip.setAttribute("transform", `translate(${boxX},${top + 6})`);
    tooltip.dataset.index = String(index);
    tooltip.innerHTML = `<rect width="${boxWidth}" height="${boxHeight}" rx="8" fill="#173f36"/><text x="14" y="25" fill="white" font-weight="700">${escapeHtml(labels[index])}</text>${datasets.map((set, row) => `<circle cx="17" cy="${47 + row * 23}" r="3.5" fill="${set.color}"/><text x="29" y="${51 + row * 23}" fill="#edf5f2">${escapeHtml(set.label)}</text><text x="${boxWidth - 14}" y="${51 + row * 23}" text-anchor="end" fill="white" font-weight="700">${formatNumber(set.values[index])}</text>`).join("")}`;
    cursor.setAttribute("visibility", "visible");
    tooltip.setAttribute("visibility", "visible");
    svg.setAttribute("aria-label", `${ariaLabel}. ${labels[index]}. ${datasets.map((set) => `${set.label}: ${formatNumber(set.values[index])}`).join(". ")}`);
  }
  function hide() {
    cursor.setAttribute("visibility", "hidden");
    tooltip.setAttribute("visibility", "hidden");
  }
  function point(event) {
    const bounds = svg.getBoundingClientRect();
    const px = (event.clientX - bounds.left) * width / bounds.width;
    const py = (event.clientY - bounds.top) * height / bounds.height;
    if (px < left || px > right || py < top || py > bottom) { hide(); return; }
    inspect(labels.length === 1 ? 0 : Math.round((px - left) / (right - left) * (labels.length - 1)));
  }
  svg.addEventListener("pointermove", point);
  svg.addEventListener("pointerdown", point);
  svg.addEventListener("pointerleave", hide);
  svg.addEventListener("focus", () => inspect(0));
  svg.addEventListener("blur", hide);
  svg.addEventListener("keydown", (event) => {
    if (!["ArrowLeft", "ArrowRight", "Home", "End", "Escape"].includes(event.key)) return;
    event.preventDefault();
    if (event.key === "Escape") hide();
    else inspect(event.key === "Home" ? 0 : event.key === "End" ? labels.length - 1 : selected + (event.key === "ArrowRight" ? 1 : -1));
  });
}

function renderTrend(target, rows, series, ariaLabel) {
  const palette = { pv: "#0f7664", uv: "#c17d11", downloads: "#2563a6", errors: "#b64141" };
  renderLineChart(target, rows.map((row) => String(row.day).slice(0, 10)), series.map((item) => ({
    label: item.label, color: palette[item.className], values: rows.map((row) => Number(row[item.field] || 0)),
  })), ariaLabel);
}

function renderCountryLines(target, rows, countries, field, label) {
  const days = [...new Set(rows.map((row) => row.day))].sort();
  const byDay = new Map(days.map((day) => [day, new Map()]));
  rows.forEach((row) => byDay.get(row.day)?.set(row.country, Number(row[field] || 0)));
  const colors = ["#0f7664", "#c17d11", "#2563a6", "#b64141", "#6d4c9b", "#2f855a"];
  renderLineChart(target, days.map((day) => day.slice(0, 10)), countries.map((country, index) => ({
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
    ["Successful requests", "downloads", formatNumber, "numeric"],
    ["Failed requests", "failures", formatNumber, "numeric"],
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

let downloadPage = { days: null, rows: [], cursor: null, generation: 0, busy: false, loaded: false };
function updateDownloadPage(page, append = false) {
  const seen = new Set(append ? downloadPage.rows.map((row) => String(row.id)) : []);
  const incoming = page.events.filter((row) => !seen.has(String(row.id)));
  downloadPage.rows = append ? [...downloadPage.rows, ...incoming] : incoming;
  downloadPage.cursor = page.next_cursor;
  downloadPage.loaded = true;
  renderDownloadEvents("downloads-events", downloadPage.rows);
  element("download-page-status").textContent = `Showing ${downloadPage.rows.length} requests.${page.next_cursor ? " Load more to see older requests." : " All requests for this period are shown."}`;
  element("download-more").hidden = !page.next_cursor;
  element("download-more").textContent = "Load more";
}

element("download-more").addEventListener("click", async () => {
  const state = downloadPage;
  if (state.busy || state.generation !== dashboardGeneration) return;
  state.busy = true;
  const button = element("download-more");
  button.disabled = true;
  element("download-page-status").textContent = "Loading requests...";
  try {
    const suffix = state.loaded && state.cursor ? `&cursor=${encodeURIComponent(state.cursor)}` : "";
    const page = await api(`/download-events-page?days=${state.days}${suffix}`);
    if (state !== downloadPage || state.generation !== dashboardGeneration) return;
    updateDownloadPage(page, state.loaded);
  } catch (error) {
    if (state !== downloadPage || state.generation !== dashboardGeneration) return;
    if (error.status === 401) { showLogin(); return; }
    element("download-page-status").textContent = `${error.message} Existing requests are kept. Retry this batch.`;
    button.textContent = "Retry this batch";
    button.hidden = false;
  } finally {
    state.busy = false;
    if (state === downloadPage) button.disabled = false;
  }
});

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
    targets.forEach((id) => { if (path === "/download-events-page" && downloadPage.rows.length) return; element(id).innerHTML = empty(`Loading ${label.toLowerCase()}…`); });
    try {
      let data;
      try { data = await api(`${path}?days=${days}`); }
      catch (requestError) {
        if (generation !== dashboardGeneration) return;
        // Retry only idempotent reads and transient transport/server failures.
        if (requestError.status && ![502, 503, 504].includes(requestError.status)) throw requestError;
        targets.forEach((id) => { if (path === "/download-events-page" && downloadPage.rows.length) return; element(id).innerHTML = empty(`Retrying ${label.toLowerCase()} after a connection problem...`); });
        data = await api(`${path}?days=${days}`);
      }
      if (generation !== dashboardGeneration) return;
      render(data);
      if (chart) chart(data);
    } catch (requestError) {
      if (generation !== dashboardGeneration) return;
      if (path === "/download-events-page") {
        element("download-more").hidden = false;
        element("download-more").textContent = "Retry this batch";
        element("download-page-status").textContent = `${requestError.message} Retry this batch without refreshing the other reports.`;
      }
      targets.forEach((id) => {
        if (path === "/download-events-page" && downloadPage.rows.length) return;
        element(id).innerHTML = empty(`${label} could not be loaded. Use Refresh to retry.`);
      });
      if (path !== "/download-events-page" || requestError.status === 401) report(label, requestError);
    }
  }
  const tasks = [];
  if (selected !== "feedback") {
    tasks.push(section("/overview", "Summary", ["overview-metrics", "traffic-metrics", "errors-metrics", "download-request-metrics"], renderOverviewMetrics));
  }
  const trends = {
    overview: ["overview-trend", [{ field: "page_views", className: "pv", label: "Page views" }, { field: "unique_visitors", className: "uv", label: "Unique visitors" }]],
    traffic: ["traffic-trend", [{ field: "page_views", className: "pv", label: "Page views" }, { field: "unique_visitors", className: "uv", label: "Unique visitors" }]],
    downloads: ["downloads-trend", [{ field: "download_page_views", className: "pv", label: "Download page views" }, { field: "downloads", className: "downloads", label: "Successful download requests" }, { field: "download_failures", className: "errors", label: "Failed download requests" }]],
    errors: ["errors-trend", [{ field: "errors", className: "errors", label: "Errors" }]],
  };
  if (trends[selected]) {
    const [target, series] = trends[selected];
    tasks.push(section("/daily", "Daily trend", selected === "downloads" ? [target, "download-visitors-trend"] : [target], () => {}, (rows) => {
      renderTrend(target, rows, series, selected === "downloads" ? "Daily download request counts" : "Daily activity");
      if (selected === "downloads") renderTrend("download-visitors-trend", rows, [
        { field: "download_page_visitors", className: "pv", label: "Download page visitors" },
        { field: "successful_visitors", className: "downloads", label: "Successful download visitors" },
      ], "Daily unique download visitors");
    }));
  }
  if (selected === "overview") {
    tasks.push(section("/pages", "Top pages", ["overview-pages"], (rows) => renderPages("overview-pages", rows)));
    tasks.push(section("/events", "Recent events", ["overview-events"], (rows) => renderEvents("overview-events", rows)));
  } else if (selected === "traffic") {
    tasks.push(section("/countries", "Countries", ["country-summary", "country-trend"], (data) => renderCountrySummary("country-summary", data.summary), (data) => renderCountryTrend("country-trend", data.daily)));
  } else if (selected === "downloads") {
    downloadPage = { days, rows: downloadPage.days === days ? downloadPage.rows : [], cursor: null, generation, busy: false, loaded: false };
    element("download-more").hidden = true;
    element("download-more").disabled = false;
    element("download-page-status").textContent = "Loading the latest 25 requests...";
    tasks.push(section("/download-events-page", "Download activity", ["downloads-events"], (page) => updateDownloadPage(page)));
    tasks.push(section("/download-summary", "Download funnel", ["downloads-funnel", "downloads-metrics"], (summary) => {
      renderMetrics("downloads-metrics", [
        ["Download page visitors", summary.page_visitors],
        ["Successful download visitors", summary.successful_visitors],
        ["Visitors with failed requests", summary.failed_visitors],
        ["No download after visit", summary.no_download_visitors],
      ]);
      const converted = summary.page_visitors - summary.no_download_visitors;
      const rate = summary.page_visitors ? (converted / summary.page_visitors * 100).toFixed(1) : "0.0";
      element("downloads-funnel").textContent = `${formatNumber(converted)} of ${formatNumber(summary.page_visitors)} download page visitors received a successful package response (${rate}%). ${formatNumber(summary.no_download_visitors)} had no successful response during this period.`;
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
