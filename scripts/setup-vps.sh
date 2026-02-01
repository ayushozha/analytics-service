#!/bin/bash
# Pulse Analytics — VPS Setup Script
# Run this via SSH on the VPS, or locally with SSH tunnels active.
#
# Prerequisites:
#   - SSH tunnel to PostgreSQL: ssh -L 5433:127.0.0.1:5433 ayush@72.62.82.57 -N
#   - SSH tunnel to Redis:      ssh -L 6380:127.0.0.1:6380 ayush@72.62.82.57 -N

set -euo pipefail

POSTGRES_HOST="${POSTGRES_HOST:-localhost}"
POSTGRES_PORT="${POSTGRES_PORT:-5433}"
POSTGRES_USER="${POSTGRES_USER:-admin}"
POSTGRES_PASSWORD="${POSTGRES_PASSWORD:-i87RfJUBx5HZJuykZt4v9u3zaq10wAqV}"

REDIS_HOST="${REDIS_HOST:-localhost}"
REDIS_PORT="${REDIS_PORT:-6380}"
REDIS_ADMIN_USER="${REDIS_ADMIN_USER:-admin}"
REDIS_ADMIN_PASS="${REDIS_ADMIN_PASS:-P0UnWC3CC7fsxV0Dsz2CgyDra19aL5iK}"

# New Redis user for Pulse Analytics
PULSE_REDIS_USER="pulse_analytics_user"
PULSE_REDIS_PASS="$(openssl rand -base64 24 | tr -d '/+=' | head -c 32)"

echo "=== Pulse Analytics VPS Setup ==="
echo ""

# Step 1: Create PostgreSQL database
echo "[1/3] Creating pulse_analytics database..."
PGPASSWORD="$POSTGRES_PASSWORD" psql \
  -h "$POSTGRES_HOST" \
  -p "$POSTGRES_PORT" \
  -U "$POSTGRES_USER" \
  -d postgres \
  -c "CREATE DATABASE pulse_analytics;" 2>/dev/null || echo "  Database already exists (OK)"

echo "  Done."

# Step 2: Create Redis ACL user
echo "[2/3] Creating Redis ACL user: $PULSE_REDIS_USER..."
echo "  Generated password: $PULSE_REDIS_PASS"
echo ""
echo "  Run this command on the VPS Redis server (via SSH):"
echo ""
echo "  redis-cli -p 6379 -u redis://$REDIS_ADMIN_USER:$REDIS_ADMIN_PASS@$REDIS_HOST:$REDIS_PORT ACL SETUSER $PULSE_REDIS_USER on >$PULSE_REDIS_PASS ~pulse_analytics:* &* +@all"
echo "  redis-cli -p 6379 -u redis://$REDIS_ADMIN_USER:$REDIS_ADMIN_PASS@$REDIS_HOST:$REDIS_PORT ACL SAVE"
echo ""

# Step 3: Generate environment file
echo "[3/3] Generating .env.production..."

PULSE_ADMIN_TOKEN="$(openssl rand -base64 32 | tr -d '/+=' | head -c 40)"

cat > .env.production << EOF
# Pulse Analytics — Production Environment
# Generated on $(date -u +"%Y-%m-%d %H:%M:%S UTC")

# Server
PULSE_PORT=8090
ENVIRONMENT=production
RUST_LOG=info

# PostgreSQL (Coolify internal network)
DATABASE_URL=postgres://admin:${POSTGRES_PASSWORD}@projects-postgres:5433/pulse_analytics?sslmode=disable

# Redis (Coolify internal network)
REDIS_URL=redis://${PULSE_REDIS_USER}:${PULSE_REDIS_PASS}@projects-redis:6379/0
REDIS_KEY_PREFIX=pulse_analytics:

# Admin
PULSE_ADMIN_TOKEN=${PULSE_ADMIN_TOKEN}

# Umami Proxy (optional)
UMAMI_URL=https://analytics.ayushojha.com
UMAMI_USER=admin
UMAMI_PASS=CHANGE_ME

# CORS (comma-separated)
ALLOWED_ORIGINS=https://ayushojha.com,https://www.ayushojha.com

# GeoIP (download from MaxMind)
GEOIP_DB_PATH=/app/data/GeoLite2-City.mmdb

# Buffer
BUFFER_FLUSH_INTERVAL_SECS=5
BUFFER_BATCH_SIZE=500
RATE_LIMIT_PER_SECOND=100
EOF

echo "  Created .env.production"
echo ""
echo "=== Setup Complete ==="
echo ""
echo "Next steps:"
echo "  1. SSH to VPS and run the Redis ACL command above"
echo "  2. Point pulse.ayushojha.com DNS A record to 72.62.82.57"
echo "  3. Update UMAMI_PASS in .env.production"
echo "  4. Deploy via Coolify using docker-compose.coolify.yml"
echo ""
echo "Credentials to save:"
echo "  PULSE_ADMIN_TOKEN: $PULSE_ADMIN_TOKEN"
echo "  REDIS_USER:        $PULSE_REDIS_USER"
echo "  REDIS_PASS:        $PULSE_REDIS_PASS"
