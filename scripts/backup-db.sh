#!/usr/bin/env bash
# ============================================
# DeeperSensor Database Backup Script
# ============================================
# Usage: ./scripts/backup-db.sh [backup_dir]

set -euo pipefail

# Configuration
BACKUP_DIR="${1:-/var/backups/deepersensor}"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
DB_NAME="${POSTGRES_DB:-deepersensor}"
DB_USER="${POSTGRES_USER:-postgres}"
DB_HOST="${POSTGRES_HOST:-localhost}"
DB_PORT="${POSTGRES_PORT:-5432}"
RETENTION_DAYS="${BACKUP_RETENTION_DAYS:-30}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Ensure backup directory exists
mkdir -p "$BACKUP_DIR"

BACKUP_FILE="$BACKUP_DIR/backup_${TIMESTAMP}.dump"
COMPRESSED_FILE="${BACKUP_FILE}.gz"

log_info "Starting database backup..."
log_info "Database: $DB_NAME@$DB_HOST:$DB_PORT"
log_info "Backup file: $COMPRESSED_FILE"

# Check if database is accessible
if ! PGPASSWORD="$POSTGRES_PASSWORD" psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" -c "SELECT 1;" >/dev/null 2>&1; then
    log_error "Cannot connect to database"
    exit 1
fi

# Perform backup using pg_dump
log_info "Creating database dump..."
if PGPASSWORD="$POSTGRES_PASSWORD" pg_dump \
    -h "$DB_HOST" \
    -p "$DB_PORT" \
    -U "$DB_USER" \
    -d "$DB_NAME" \
    -F c \
    -f "$BACKUP_FILE"; then
    log_info "Database dump created successfully"
else
    log_error "Database dump failed"
    exit 1
fi

# Compress the backup
log_info "Compressing backup..."
if gzip "$BACKUP_FILE"; then
    log_info "Backup compressed successfully"
else
    log_error "Compression failed"
    rm -f "$BACKUP_FILE"
    exit 1
fi

# Calculate size
BACKUP_SIZE=$(du -h "$COMPRESSED_FILE" | cut -f1)
log_info "Backup size: $BACKUP_SIZE"

# Rotate old backups
log_info "Rotating old backups (keeping last $RETENTION_DAYS days)..."
DELETED_COUNT=$(find "$BACKUP_DIR" -name "backup_*.dump.gz" -mtime +"$RETENTION_DAYS" -delete -print | wc -l)
log_info "Deleted $DELETED_COUNT old backup(s)"

# List recent backups
log_info "Recent backups:"
find "$BACKUP_DIR" -name "backup_*.dump.gz" -mtime -7 -printf "%T@ %p\n" | sort -rn | cut -d' ' -f2- | head -5

log_info "Backup completed successfully: $COMPRESSED_FILE"

# Optional: Upload to cloud storage
if [ -n "${S3_BACKUP_BUCKET:-}" ]; then
    log_info "Uploading to S3..."
    aws s3 cp "$COMPRESSED_FILE" "s3://$S3_BACKUP_BUCKET/backups/$(basename "$COMPRESSED_FILE")"
    log_info "Uploaded to S3"
fi

exit 0
