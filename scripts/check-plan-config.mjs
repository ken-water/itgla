import fs from "node:fs";

const config = JSON.parse(fs.readFileSync(new URL("../config/plans.json", import.meta.url), "utf8"));
const html = fs.readFileSync(new URL("../website/plans.html", import.meta.url), "utf8");
const free = config.plans.free;
const pro = config.plans.pro;
const team = config.plans.team;
if (config.status !== "free_first" || config.limits_enabled || config.public_pricing) {
  throw new Error("plans.json must keep free-first mode with limits and public pricing disabled");
}
for (const value of ["has no asset-count limit", "no paid plan or price is available today", "No current limit"]) {
  if (!html.includes(value)) throw new Error(`website/plans.html missing free-first copy: ${value}`);
}
if (html.includes("$19/month") || html.includes("$49/month") || html.includes("2,000")) {
  throw new Error("website/plans.html exposes unpublished pricing or future capacity");
}
console.log("Plan configuration and website copy agree.");
