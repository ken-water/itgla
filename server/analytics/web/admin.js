const element = (id) => document.getElementById(id);

async function api(path, options = {}) {
  const response = await fetch(`/admin/api${path}`, {
    credentials: "same-origin",
    headers: { "Content-Type": "application/json", ...(options.headers || {}) },
    ...options,
  });
  const body = await response.json().catch(() => ({}));
  if (!response.ok) {
    const error = new Error(body.message || "请求失败，请稍后重试。");
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
  return Number(value || 0).toLocaleString("zh-CN");
}

function formatTime(value) {
  return value ? new Date(value).toLocaleString("zh-CN", { hour12: false }) : "暂无";
}

function empty(message) {
  return `<p class="empty">${escapeHtml(message)}</p>`;
}

function table(rows, columns) {
  if (!rows.length) return empty("当前周期暂无数据。");
  return `<table><thead><tr>${columns.map(([label]) => `<th>${escapeHtml(label)}</th>`).join("")}</tr></thead><tbody>${rows.map((row) => `<tr>${columns.map(([, key, formatter, className]) => `<td class="${className || ""}">${escapeHtml(formatter ? formatter(row[key]) : row[key])}</td>`).join("")}</tr>`).join("")}</tbody></table>`;
}

function renderMetrics(overview) {
  const items = [
    ["页面浏览（PV）", overview.page_views],
    ["独立访客（UV）", overview.unique_visitors],
    ["安装包下载", overview.downloads],
    ["页面错误", overview.errors],
  ];
  element("metrics").innerHTML = items.map(([label, value]) => `<article class="metric"><span>${label}</span><strong>${formatNumber(value)}</strong></article>`).join("");
  element("freshness").textContent = overview.latest_event_at ? `最新数据：${formatTime(overview.latest_event_at)}` : "当前还没有访问记录";
}

function renderTrend(rows) {
  const container = element("trend");
  if (!rows.length) {
    container.innerHTML = empty("访问产生后，这里会显示每日 PV 与 UV 趋势。");
    return;
  }
  const width = 760;
  const height = 270;
  const padding = { top: 18, right: 16, bottom: 34, left: 38 };
  const maximum = Math.max(1, ...rows.flatMap((row) => [Number(row.page_views), Number(row.unique_visitors)]));
  const x = (index) => padding.left + (index * (width - padding.left - padding.right)) / Math.max(1, rows.length - 1);
  const y = (value) => height - padding.bottom - (Number(value) * (height - padding.top - padding.bottom)) / maximum;
  const points = (field) => rows.map((row, index) => `${x(index)},${y(row[field])}`).join(" ");
  const grids = [0, .5, 1].map((ratio) => `<line class="chart-grid" x1="${padding.left}" x2="${width - padding.right}" y1="${y(maximum * ratio)}" y2="${y(maximum * ratio)}"></line><text class="chart-label" x="2" y="${y(maximum * ratio) + 4}">${Math.round(maximum * ratio)}</text>`).join("");
  const labels = rows.map((row, index) => {
    const interval = Math.max(1, Math.ceil(rows.length / 6));
    if (index % interval !== 0 && index !== rows.length - 1) return "";
    return `<text class="chart-label" text-anchor="middle" x="${x(index)}" y="${height - 7}">${escapeHtml(String(row.day).slice(5))}</text>`;
  }).join("");
  container.innerHTML = `<svg viewBox="0 0 ${width} ${height}" role="img" aria-label="每日页面浏览和独立访客折线图">${grids}<polyline class="chart-pv" points="${points("page_views")}"></polyline><polyline class="chart-uv" points="${points("unique_visitors")}"></polyline>${labels}</svg>`;
}

function renderPages(rows) {
  element("pages").innerHTML = table(rows, [
    ["页面", "path"], ["PV", "page_views", formatNumber, "numeric"], ["UV", "unique_visitors", formatNumber, "numeric"],
  ]);
}

function renderEvents(rows) {
  const names = { page_view: "页面浏览", download: "下载", page_error: "页面错误" };
  element("events").innerHTML = table(rows, [
    ["时间", "occurred_at", formatTime], ["类型", "event_name", (value) => names[value] || value], ["路径", "path"], ["状态", "status_code", formatNumber, "numeric"], ["来源", "referrer", (value) => value && value !== "-" ? value : "直接访问"],
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
    renderMetrics(overview);
    renderTrend(daily);
    renderPages(pages);
    renderEvents(events);
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
element("logout").addEventListener("click", async () => {
  await api("/logout", { method: "POST" }).catch(() => {});
  showLogin();
});

api("/me").then((session) => session.authenticated ? showDashboard() : showLogin()).catch(showLogin);
