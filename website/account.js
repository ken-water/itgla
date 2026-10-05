const accountStatus = document.getElementById("account-status");
const list = document.getElementById("server-list");
const form = document.getElementById("server-form");
const search = document.getElementById("server-search");
const status = document.getElementById("workspace-status");
const count = document.getElementById("server-count");
const tagCount = document.getElementById("tag-count");
let servers = [];
function escapeHtml(value) { return String(value).replace(/[&<>"']/g, (character) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[character]); }
function render() {
  const query = search.value.trim().toLowerCase();
  const visible = servers.filter((server) => [server.name, server.tags, server.ip_address, server.ports].join(" ").toLowerCase().includes(query));
  count.textContent = String(servers.length);
  tagCount.textContent = String(new Set(servers.flatMap((server) => server.tags.split(",").map((tag) => tag.trim()).filter(Boolean))).size);
  list.innerHTML = visible.length ? visible.map((server) => `<article class="server-item"><div><h3>${escapeHtml(server.name)}</h3><p>${escapeHtml(server.tags || "No tags")}</p></div><div class="server-meta"><code>${escapeHtml(server.ip_address || "No IP")}</code><code>${escapeHtml(server.ports || "No ports")}</code></div><button class="icon-button" data-delete="${server.id}" aria-label="Delete ${escapeHtml(server.name)}">×</button></article>`).join("") : `<p class="empty-state">${query ? "No servers match this search." : "Your quick list is empty. Add the first server."}</p>`;
}
async function load() { const response = await fetch("/api/workspace/servers", { credentials: "same-origin" }); if (response.status === 401) return location.replace("/signin.html"); if (!response.ok) throw new Error("Unable to load your workspace."); servers = await response.json(); render(); }
document.querySelector("#new-server").addEventListener("click", () => form.elements.name.focus());
search.addEventListener("input", render);
list.addEventListener("click", async (event) => { const button = event.target.closest("[data-delete]"); if (!button) return; const response = await fetch(`/api/workspace/servers/${button.dataset.delete}`, { method: "DELETE", credentials: "same-origin" }); if (response.ok) { status.textContent = "Server removed."; await load(); } });
form.addEventListener("submit", async (event) => { event.preventDefault(); const button = form.querySelector("button[type=submit]"); button.disabled = true; status.textContent = "Saving…"; try { const response = await fetch("/api/workspace/servers", { method: "POST", credentials: "same-origin", headers: { "Content-Type": "application/json" }, body: JSON.stringify(Object.fromEntries(new FormData(form))) }); const body = await response.json(); if (!response.ok) throw new Error(body.message || "Unable to save server."); form.reset(); status.textContent = "Server saved."; await load(); } catch (error) { status.textContent = error.message; } finally { button.disabled = false; } });
fetch("/auth/me", { credentials: "same-origin" }).then((response) => response.json()).then(async (body) => { if (!body.authenticated) return location.replace("/signin.html"); accountStatus.textContent = body.email; await load(); }).catch(() => location.replace("/signin.html"));
