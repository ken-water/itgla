import { readFile, readdir } from "node:fs/promises";
import crypto from "node:crypto";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..", "website");
const files = (await readdir(root))
  .filter((file) => file.endsWith(".html"))
  .map((file) => resolve(root, file))
  .sort();
const pattern = /<footer class="site-footer compact-footer">[\s\S]*?<\/footer>/;
const hashes = new Map();
const failures = [];

for (const file of files) {
  const source = await readFile(file, "utf8");
  const match = source.match(pattern);
  if (!match) {
    failures.push(`${file}: missing unified footer`);
    continue;
  }
  hashes.set(file, crypto.createHash("sha256").update(match[0]).digest("hex"));
}

const unique = new Set(hashes.values());
if (unique.size !== 1 || failures.length > 0) {
  for (const failure of failures) console.error(failure);
  console.error(`Footer consistency failed: ${hashes.size} pages, ${unique.size} footer variants.`);
  process.exit(1);
}

console.log(`Footer consistency passed: ${hashes.size} pages, one shared footer.`);
