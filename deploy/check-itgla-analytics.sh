#!/usr/bin/env bash
set -euo pipefail

BASE_URL="${ITGLA_BASE_URL:-https://itgla.com}"
ADMIN_USERNAME="${ITGLA_ADMIN_USERNAME:?Set ITGLA_ADMIN_USERNAME}"
ADMIN_PASSWORD="${ITGLA_ADMIN_PASSWORD:?Set ITGLA_ADMIN_PASSWORD}"
COOKIE_JAR="$(mktemp /tmp/itgla-admin-cookie.XXXXXX)"
trap 'rm -f "$COOKIE_JAR"' EXIT

test "$(curl -fsS -o /dev/null -w '%{http_code}' "$BASE_URL/admin/")" = "200"
test "$(curl -sS -o /dev/null -w '%{http_code}' "$BASE_URL/admin/api/overview")" = "401"
curl -fsS -c "$COOKIE_JAR" -H 'Content-Type: application/json' \
  --data "$(ADMIN_USERNAME="$ADMIN_USERNAME" ADMIN_PASSWORD="$ADMIN_PASSWORD" node -e 'process.stdout.write(JSON.stringify({username:process.env.ADMIN_USERNAME,password:process.env.ADMIN_PASSWORD}))')" \
  "$BASE_URL/admin/api/login" >/dev/null
curl -fsS -b "$COOKIE_JAR" "$BASE_URL/admin/api/overview?days=30" | node -e '
let body="";process.stdin.on("data",chunk=>body+=chunk);process.stdin.on("end",()=>{
  const value=JSON.parse(body);
  for(const key of ["page_views","unique_visitors","downloads","errors"]){
    if(!Number.isInteger(value[key])) throw new Error(`missing metric: ${key}`);
  }
});'
curl -fsS -b "$COOKIE_JAR" -H 'Content-Type: application/json' -X POST "$BASE_URL/admin/api/logout" >/dev/null
echo "ITGLA analytics checks passed"
