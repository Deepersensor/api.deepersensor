#!/usr/bin/env bash
# ============================================
# DeeperSensor Database Restore Script
# ============================================
# Usage: ./scripts/restore-db.sh <backup_file>

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_info() { echo -e "${GREEN}[INFO]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

# Check arguments
if [ $# -lt 1 ]; then
    log_error "Usage: $0 <backup_file.dump.gz>"
    log_error "Example: $0 /var/backups/deepersensor/backup_20250929_020000.dump.gz"
    exit 1
fi

BACKUP_FILE="$1"
DB_NAME="${POSTGRES_DB:-deepersensor}"
DB_USER="${POSTGRES_USER:-postgres}"
DB_HOST="${POSTGRES_HOST:-localhost}"
DB_PORT="${POSTGRES_PORT:-5432}"

# Verify backup file exists
if [ ! -f "$BACKUP_FILE" ]; then
    log_error "Backup file not found: $BACKUP_FILE"
    exit 1
fi

log_warn "╔═════════════════════════════════════════════════════════════╗"
log_warn "║              DATABASE RESTORE WARNING                       ║"
log_warn "╠═════════════════════════════════════════════════════════════╣"
log_warn "║ This will DESTROY all current data in the database!        ║"
log_warn "║ Database: $DB_NAME@$DB_HOST:$DB_PORT"
log_warn "║ Backup file: $(basename "$BACKUP_FILE")"
log_warn "╚═════════════════════════════════════════════════════════════╝"

read -p "Type 'RESTORE' to confirm: " CONFIRM
if [ "$CONFIRM" != "RESTORE" ]; then
    log_error "Restore cancelled"
    exit 1
fi

# Stop API if running
if docker ps --format '{{.Names}}' | grep -q "deepersensor-api"; then
    log_warn "Stopping API container..."
    docker stop deepersensor-api
    API_WAS_RUNNING=true
else
    API_WAS_RUNNING=false
fi

log_info "Starting database restore..."

# Decompress if needed
RESTORE_FILE="$BACKUP_FILE"
if [[ "$BACKUP_FILE" == *.gz ]]; then
    log_info "Decompressing backup..."
    RESTORE_FILE="/tmp/restore_temp.dump"
    gunzip -c "$BACKUP_FILE" > "$RESTORE_FILE"
fi

# Restore database
log_info "Restoring database..."
if PGPASSWORD="$POSTGRES_PASSWORD" pg_restore \
    -h "$DB_HOST" \
    -p "$DB_PORT" \
    -U "$DB_USER" \
    -d "$DB_NAME" \
    --clean \
    --if-exists \
    --no-owner \
    --no-acl \
    "$RESTORE_FILE"; then
    log_info "Database restored successfully"
else
    log_error "Restore failed!"
    # Cleanup temp file
    [ "$RESTORE_FILE" != "$BACKUP_FILE" ] && rm -f "$RESTORE_FILE"
    exit 1
fi

# Cleanup temp file
if [ "$RESTORE_FILE" != "$BACKUP_FILE" ]; then
    rm -f "$RESTORE_FILE"
fi

# Verify restore
log_info "Verifying restore..."
if PGPASSWORD="$POSTGRES_PASSWORD" psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" -c "SELECT COUNT(*) FROM users;" >/dev/null 2>&1; then
    log_info "Verification successful"
else
    log_warn "Verification check failed - database may be in an inconsistent state"
fi

# Restart API if it was running
if [ "$API_WAS_RUNNING" = true ]; then
    log_info "Restarting API container..."
    docker start deepersensor-api
fi

log_info "Restore completed"
exit 0
