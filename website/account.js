const accountStatus = document.getElementById("account-status");
const list = document.getElementById("server-list");
const form = document.getElementById("server-form");
const search = document.getElementById("server-search");
const tagFilter = document.getElementById("tag-filter");
const status = document.getElementById("workspace-status");
const count = document.getElementById("server-count");
const tagCount = document.getElementById("tag-count");
const sortButton = document.getElementById("sort-name");
const cancelEdit = document.getElementById("cancel-edit");
const formTitle = document.getElementById("form-title");
const formEyebrow = document.getElementById("form-eyebrow");
const saveButton = document.getElementById("save-server");
let servers = [];
let editingId = null;
let sortAscending = true;

function escapeHtml(value) {
  return String(value).replace(/[&<>"']/g, (character) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[character]);
}
function tagsFor(server) {
  return server.tags.split(",").map((tag) => tag.trim()).filter(Boolean);
}
function copyValue(value) {
  if (!value) return;
  navigator.clipboard.writeText(value).then(() => { status.textContent = "Copied."; }).catch(() => { status.textContent = "Copy was blocked by the browser."; });
}
function refreshTagFilter() {
  const selected = tagFilter.value;
  const tags = [...new Set(servers.flatMap(tagsFor))].sort((a, b) => a.localeCompare(b));
  tagFilter.innerHTML = `<option value="">All tags</option>${tags.map((tag) => `<option value="${escapeHtml(tag)}">${escapeHtml(tag)}</option>`).join("")}`;
  tagFilter.value = tags.includes(selected) ? selected : "";
}
function render() {
  const query = search.value.trim().toLowerCase();
  const selectedTag = tagFilter.value.toLowerCase();
  const visible = servers
    .filter((server) => [server.name, server.tags, server.ip_address, server.ports].join(" ").toLowerCase().includes(query))
    .filter((server) => !selectedTag || tagsFor(server).some((tag) => tag.toLowerCase() === selectedTag))
    .sort((a, b) => a.name.localeCompare(b.name) * (sortAscending ? 1 : -1));
  count.textContent = String(servers.length);
  tagCount.textContent = String(new Set(servers.flatMap(tagsFor)).size);
  list.innerHTML = visible.length
    ? `<div class="server-table" role="table" aria-label="Your servers"><div class="server-row server-row-head" role="row"><span role="columnheader">Name</span><span role="columnheader">Tags</span><span role="columnheader">IP address</span><span role="columnheader">Ports</span><span role="columnheader">Actions</span></div>${visible.map((server) => `<div class="server-row" role="row"><span class="server-name" role="cell">${escapeHtml(server.name)}</span><span role="cell">${escapeHtml(server.tags || "No tags")}</span><span role="cell" class="copy-cell"><code>${escapeHtml(server.ip_address || "No IP")}</code>${server.ip_address ? `<button class="copy-button" data-copy="${escapeHtml(server.ip_address)}" aria-label="Copy IP address">⧉</button>` : ""}</span><span role="cell" class="copy-cell"><code>${escapeHtml(server.ports || "No ports")}</code>${server.ports ? `<button class="copy-button" data-copy="${escapeHtml(server.ports)}" aria-label="Copy ports">⧉</button>` : ""}</span><span role="cell" class="row-actions"><button class="text-button" data-edit="${server.id}" type="button">Edit</button><button class="text-button danger" data-delete="${server.id}" type="button">Delete</button></span></div>`).join("")}</div>`
    : `<p class="empty-state">${query || selectedTag ? "No servers match this filter." : "Your quick list is empty. Add the first server."}</p>`;
}
function beginEdit(server) {
  if (!server) return;
  editingId = server.id;
  formTitle.textContent = "Edit server";
  formEyebrow.textContent = "Update record";
  saveButton.textContent = "Save changes";
  cancelEdit.hidden = false;
  ["name", "tags", "ip_address", "ports"].forEach((key) => { form.elements[key].value = server[key] || ""; });
  form.elements.name.focus();
}
function resetForm() {
  editingId = null;
  form.reset();
  formTitle.textContent = "Add a server";
  formEyebrow.textContent = "New record";
  saveButton.textContent = "Save server";
  cancelEdit.hidden = true;
}
async function load() {
  const response = await fetch("/api/workspace/servers", { credentials: "same-origin" });
  if (response.status === 401) return location.replace("/signin.html");
  if (!response.ok) throw new Error("Unable to load your workspace.");
  servers = await response.json();
  refreshTagFilter();
  render();
}
document.querySelector("#new-server").addEventListener("click", () => form.elements.name.focus());
search.addEventListener("input", render);
tagFilter.addEventListener("change", render);
sortButton.addEventListener("click", () => { sortAscending = !sortAscending; sortButton.innerHTML = `Name ${sortAscending ? "↑" : "↓"}`; render(); });
cancelEdit.addEventListener("click", resetForm);
list.addEventListener("click", async (event) => {
  const copy = event.target.closest("[data-copy]");
  if (copy) return copyValue(copy.dataset.copy);
  const edit = event.target.closest("[data-edit]");
  if (edit) return beginEdit(servers.find((server) => String(server.id) === edit.dataset.edit));
  const button = event.target.closest("[data-delete]");
  if (!button || !window.confirm("Delete this server record?")) return;
  const response = await fetch(`/api/workspace/servers/${button.dataset.delete}`, { method: "DELETE", credentials: "same-origin" });
  if (response.ok) { status.textContent = "Server removed."; await load(); }
});
form.addEventListener("submit", async (event) => {
  event.preventDefault();
  saveButton.disabled = true;
  status.textContent = editingId ? "Saving changes…" : "Saving…";
  try {
    const response = await fetch(editingId ? `/api/workspace/servers/${editingId}` : "/api/workspace/servers", {
      method: editingId ? "PATCH" : "POST",
      credentials: "same-origin",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(Object.fromEntries(new FormData(form))),
    });
    const body = await response.json();
    if (!response.ok) throw new Error(body.message || "Unable to save server.");
    status.textContent = editingId ? "Server updated." : "Server saved.";
    resetForm();
    await load();
  } catch (error) {
    status.textContent = error.message;
  } finally {
    saveButton.disabled = false;
  }
});
fetch("/auth/me", { credentials: "same-origin" }).then((response) => response.json()).then(async (body) => {
  if (!body.authenticated) return location.replace("/signin.html");
  accountStatus.textContent = body.email;
  await load();
}).catch(() => location.replace("/signin.html"));
