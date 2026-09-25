import fs from "node:fs";
import path from "node:path";
import process from "node:process";

const root = path.resolve(import.meta.dirname, "..");
const targets = ["ui/app.slint", "src", "website", "server/analytics", "README.md", "CHANGELOG.md", "docs"];
const extensions = new Set([".html", ".js", ".mjs", ".md", ".rs", ".slint"]);
const forbidden = /[\p{Script=Han}\p{Script=Hiragana}\p{Script=Katakana}\p{Script=Hangul}]/u;
const ignoredDirectories = new Set(["node_modules"]);
const failures = [];

function scan(relativePath) {
  const absolutePath = path.join(root, relativePath);
  const stat = fs.statSync(absolutePath);
  if (stat.isDirectory()) {
    for (const entry of fs.readdirSync(absolutePath, { withFileTypes: true })) {
      if (!ignoredDirectories.has(entry.name)) scan(path.join(relativePath, entry.name));
    }
    return;
  }
  if (!extensions.has(path.extname(relativePath))) return;
  fs.readFileSync(absolutePath, "utf8").split(/\r?\n/u).forEach((line, index) => {
    if (forbidden.test(line)) failures.push(`${relativePath}:${index + 1}`);
  });
}

targets.forEach(scan);
if (failures.length) {
  console.error(`English-only UI gate failed:\n${failures.join("\n")}`);
  process.exitCode = 1;
} else {
  console.log("English-only UI gate passed.");
}
