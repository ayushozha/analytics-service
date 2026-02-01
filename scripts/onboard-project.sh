#!/usr/bin/env bash
set -euo pipefail

# ─────────────────────────────────────────────────────────────
# Pulse Analytics — Project Onboarding Script
# Creates a new project with API keys and outputs integration snippets
# ─────────────────────────────────────────────────────────────

PULSE_URL="${PULSE_URL:-}"
PULSE_ADMIN_TOKEN="${PULSE_ADMIN_TOKEN:-}"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

info() { echo -e "${CYAN}[INFO]${NC} $1"; }
success() { echo -e "${GREEN}[OK]${NC} $1"; }
error() { echo -e "${RED}[ERROR]${NC} $1" >&2; }

# Check dependencies
for cmd in curl jq; do
    if ! command -v "$cmd" &>/dev/null; then
        error "$cmd is required but not installed"
        exit 1
    fi
done

# Prompt for config if not set
if [[ -z "$PULSE_URL" ]]; then
    read -rp "Pulse Analytics URL (e.g. https://pulse.ayushojha.com): " PULSE_URL
fi
if [[ -z "$PULSE_ADMIN_TOKEN" ]]; then
    read -rsp "Admin token (PULSE_ADMIN_TOKEN): " PULSE_ADMIN_TOKEN
    echo
fi

# Verify connection
info "Verifying connection to ${PULSE_URL}..."
HEALTH=$(curl -sf "${PULSE_URL}/health" 2>/dev/null || true)
if [[ -z "$HEALTH" ]]; then
    error "Cannot reach ${PULSE_URL}/health"
    exit 1
fi
success "Connected to Pulse Analytics"

# Project details
echo ""
read -rp "Project name: " PROJECT_NAME
read -rp "Project domain (e.g. example.com): " PROJECT_DOMAIN

# Create project
info "Creating project '${PROJECT_NAME}'..."
PROJECT_RESPONSE=$(curl -sf -X POST "${PULSE_URL}/api/admin/projects" \
    -H "Authorization: Bearer ${PULSE_ADMIN_TOKEN}" \
    -H "Content-Type: application/json" \
    -d "{\"name\": \"${PROJECT_NAME}\", \"domain\": \"${PROJECT_DOMAIN}\"}")

if [[ -z "$PROJECT_RESPONSE" ]]; then
    error "Failed to create project"
    exit 1
fi

PROJECT_ID=$(echo "$PROJECT_RESPONSE" | jq -r '.id')
success "Project created: ${PROJECT_ID}"

# Create ingest API key
info "Creating ingest API key..."
INGEST_KEY_RESPONSE=$(curl -sf -X POST "${PULSE_URL}/api/admin/projects/${PROJECT_ID}/keys" \
    -H "Authorization: Bearer ${PULSE_ADMIN_TOKEN}" \
    -H "Content-Type: application/json" \
    -d "{\"name\": \"${PROJECT_NAME}-ingest\", \"scopes\": [\"ingest\"]}")

INGEST_KEY=$(echo "$INGEST_KEY_RESPONSE" | jq -r '.key')
success "Ingest key created"

# Create query API key (for dashboard access)
info "Creating query API key..."
QUERY_KEY_RESPONSE=$(curl -sf -X POST "${PULSE_URL}/api/admin/projects/${PROJECT_ID}/keys" \
    -H "Authorization: Bearer ${PULSE_ADMIN_TOKEN}" \
    -H "Content-Type: application/json" \
    -d "{\"name\": \"${PROJECT_NAME}-query\", \"scopes\": [\"query\"]}")

QUERY_KEY=$(echo "$QUERY_KEY_RESPONSE" | jq -r '.key')
success "Query key created"

# Output integration snippets
echo ""
echo -e "${BOLD}═══════════════════════════════════════════════════════════${NC}"
echo -e "${BOLD}  Project Onboarded Successfully${NC}"
echo -e "${BOLD}═══════════════════════════════════════════════════════════${NC}"
echo ""
echo -e "${BOLD}Project ID:${NC}  ${PROJECT_ID}"
echo -e "${BOLD}Ingest Key:${NC}  ${INGEST_KEY}"
echo -e "${BOLD}Query Key:${NC}   ${QUERY_KEY}"
echo ""

echo -e "${BOLD}── Script Tag (paste in <head>) ──${NC}"
echo ""
echo "  <script defer"
echo "    src=\"${PULSE_URL}/api/script.js\""
echo "    data-key=\"${INGEST_KEY}\""
echo "    data-api=\"${PULSE_URL}\"></script>"
echo ""

echo -e "${BOLD}── TypeScript SDK ──${NC}"
echo ""
echo "  import { createPulse } from '@ayushojha/pulse-analytics';"
echo ""
echo "  const pulse = createPulse({"
echo "    apiKey: '${INGEST_KEY}',"
echo "    apiUrl: '${PULSE_URL}',"
echo "  });"
echo ""

echo -e "${BOLD}── Next.js Environment Variables ──${NC}"
echo ""
echo "  NEXT_PUBLIC_PULSE_URL=${PULSE_URL}"
echo "  NEXT_PUBLIC_PULSE_KEY=${INGEST_KEY}"
echo ""

echo -e "${BOLD}── Dashboard ──${NC}"
echo ""
echo "  URL:  ${PULSE_URL}/dashboard"
echo "  Key:  ${QUERY_KEY} (use this to log in)"
echo ""

# Optional: Redis ACL setup
if [[ "${1:-}" == "--with-redis-acl" ]]; then
    VPS_HOST="${VPS_HOST:-72.62.82.57}"
    VPS_USER="${VPS_USER:-ayush}"
    REDIS_ADMIN_PASS="${REDIS_ADMIN_PASS:-P0UnWC3CC7fsxV0Dsz2CgyDra19aL5iK}"

    REDIS_USER=$(echo "${PROJECT_NAME}" | tr '-' '_' | tr ' ' '_' | tr '[:upper:]' '[:lower:]')_user
    REDIS_PASS=$(openssl rand -base64 24 | tr -d '=+/')
    REDIS_PREFIX=$(echo "${PROJECT_NAME}" | tr '-' '_' | tr ' ' '_' | tr '[:upper:]' '[:lower:]'):

    info "Creating Redis ACL user '${REDIS_USER}' on VPS..."
    ssh "${VPS_USER}@${VPS_HOST}" "redis-cli -p 6379 -a '${REDIS_ADMIN_PASS}' --no-auth-warning ACL SETUSER ${REDIS_USER} on \\>${REDIS_PASS} ~${REDIS_PREFIX}* &* +@all && redis-cli -p 6379 -a '${REDIS_ADMIN_PASS}' --no-auth-warning ACL SAVE"

    echo -e "${BOLD}── Redis Credentials ──${NC}"
    echo ""
    echo "  Username:   ${REDIS_USER}"
    echo "  Password:   ${REDIS_PASS}"
    echo "  Key Prefix: ${REDIS_PREFIX}"
    echo "  URL:        redis://${REDIS_USER}:${REDIS_PASS}@localhost:6380/0"
    echo ""
fi

echo -e "${GREEN}Done! Save these credentials securely — API keys are shown only once.${NC}"
