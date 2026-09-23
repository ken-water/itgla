#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REMOTE_HOST="${ITGLA_HOST:?Set ITGLA_HOST, for example root@186.244.233.223}"
ADMIN_USERNAME="${ITGLA_ADMIN_USERNAME:?Set ITGLA_ADMIN_USERNAME}"
ADMIN_PASSWORD="${ITGLA_ADMIN_PASSWORD:?Set ITGLA_ADMIN_PASSWORD}"
REMOTE_ROOT="/opt/itgla-analytics"

if [[ ! "$ADMIN_USERNAME" =~ ^[A-Za-z0-9._-]+$ ]]; then
  echo "ITGLA_ADMIN_USERNAME may contain only letters, numbers, dot, underscore, and hyphen" >&2
  exit 1
fi
if (( ${#ADMIN_PASSWORD} < 10 )); then
  echo "ITGLA_ADMIN_PASSWORD must contain at least 10 characters" >&2
  exit 1
fi

DATABASE_PASSWORD="$(openssl rand -hex 32)"
PASSWORD_SALT="$(openssl rand -hex 24)"
VISITOR_SALT="$(openssl rand -hex 32)"
PASSWORD_SCRYPT="$(PASSWORD_VALUE="$ADMIN_PASSWORD" PASSWORD_SALT="$PASSWORD_SALT" node -e 'const c=require("node:crypto");process.stdout.write(c.scryptSync(process.env.PASSWORD_VALUE,process.env.PASSWORD_SALT,64).toString("hex"))')"
ENV_FILE="$(mktemp /tmp/itgla-analytics-env.XXXXXX)"
trap 'rm -f "$ENV_FILE"' EXIT
chmod 600 "$ENV_FILE"
printf '%s\n' \
  "NODE_ENV=production" \
  "ITGLA_ANALYTICS_PORT=3420" \
  "ITGLA_ACCESS_LOG=/var/log/itgla/nginx-access.log" \
  "ITGLA_DATABASE_URL=postgresql://itgla_analytics:${DATABASE_PASSWORD}@127.0.0.1/itgla_analytics" \
  "ITGLA_ADMIN_USERNAME=${ADMIN_USERNAME}" \
  "ITGLA_ADMIN_PASSWORD_SALT=${PASSWORD_SALT}" \
  "ITGLA_ADMIN_PASSWORD_SCRYPT=${PASSWORD_SCRYPT}" \
  "ITGLA_VISITOR_SALT=${VISITOR_SALT}" \
  "ITGLA_RETENTION_DAYS=365" > "$ENV_FILE"

ssh -o BatchMode=yes "$REMOTE_HOST" "set -eu
id -u itgla >/dev/null 2>&1 || useradd --system --home-dir '$REMOTE_ROOT' --shell /usr/sbin/nologin itgla
install -d -o itgla -g itgla -m 0755 '$REMOTE_ROOT'
install -d -o root -g root -m 0755 /etc/itgla
install -d -o www-data -g itgla -m 0750 /var/log/itgla
touch /var/log/itgla/nginx-access.log
chown www-data:itgla /var/log/itgla/nginx-access.log
chmod 0640 /var/log/itgla/nginx-access.log
"

rsync -a --delete --exclude node_modules/ "$ROOT_DIR/server/analytics/" "$REMOTE_HOST:$REMOTE_ROOT/"
scp "$ROOT_DIR/deploy/systemd/itgla-analytics.service" "$REMOTE_HOST:/etc/systemd/system/itgla-analytics.service"
scp "$ROOT_DIR/deploy/logrotate/itgla-analytics" "$REMOTE_HOST:/etc/logrotate.d/itgla-analytics"
scp "$ENV_FILE" "$REMOTE_HOST:/tmp/itgla-analytics.env"

ssh -o BatchMode=yes "$REMOTE_HOST" "set -eu
if sudo -u postgres psql -Atqc \"select 1 from pg_roles where rolname='itgla_analytics'\" | grep -qx 1; then
  sudo -u postgres psql -v ON_ERROR_STOP=1 -c \"alter role itgla_analytics with login nosuperuser nocreatedb nocreaterole noreplication password '$DATABASE_PASSWORD'\"
else
  sudo -u postgres psql -v ON_ERROR_STOP=1 -c \"create role itgla_analytics login nosuperuser nocreatedb nocreaterole noreplication password '$DATABASE_PASSWORD'\"
fi
sudo -u postgres psql -Atqc \"select 1 from pg_database where datname='itgla_analytics'\" | grep -qx 1 || sudo -u postgres createdb -O itgla_analytics itgla_analytics
install -o root -g root -m 0600 /tmp/itgla-analytics.env /etc/itgla/analytics.env
chown root:root /etc/logrotate.d/itgla-analytics
chmod 0644 /etc/logrotate.d/itgla-analytics
rm -f /tmp/itgla-analytics.env
cd '$REMOTE_ROOT'
npm ci --omit=dev --no-audit --no-fund
chown -R itgla:itgla '$REMOTE_ROOT'
find '$REMOTE_ROOT' -type d -exec chmod 0755 {} +
find '$REMOTE_ROOT' -type f -exec chmod 0644 {} +
systemctl daemon-reload
systemctl enable itgla-analytics
systemctl restart itgla-analytics
"

echo "ITGLA analytics service installed on $REMOTE_HOST"
