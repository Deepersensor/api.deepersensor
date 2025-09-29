# DeeperSensor API - Deployment Guide

> **Production-Ready Deployment Documentation**  
> Last Updated: September 2025

This guide covers deploying the DeeperSensor API to production environments with proper security, monitoring, and operational practices.

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Environment Configuration](#environment-configuration)
3. [Deployment Strategies](#deployment-strategies)
4. [Database Management](#database-management)
5. [Security Hardening](#security-hardening)
6. [Monitoring & Observability](#monitoring--observability)
7. [Backup & Recovery](#backup--recovery)
8. [Scaling & Performance](#scaling--performance)
9. [Troubleshooting](#troubleshooting)

---

## Prerequisites

### Infrastructure Requirements

**Minimum Production Specs:**
- **Compute:** 2 vCPUs, 4GB RAM (API + Postgres + Redis + Ollama)
- **Storage:** 20GB SSD (system) + 50GB (Ollama models)
- **Network:** Static IP or domain with TLS certificate
- **OS:** Ubuntu 22.04 LTS / Debian 12 / RHEL 9 (or compatible)

**Recommended Production Specs:**
- **Compute:** 4 vCPUs, 8GB RAM
- **Storage:** 50GB SSD (system) + 100GB (models) + separate volume for DB
- **Load Balancer:** Cloudflare, AWS ALB, or similar with TLS termination
- **Database:** Managed Postgres (AWS RDS, DigitalOcean, etc.) with automated backups

### Required Software

- Docker 24.0+ and Docker Compose 2.20+
- Git 2.30+
- A reverse proxy with TLS (nginx, Caddy, Traefik)
- Monitoring tools (Prometheus, Grafana - optional but recommended)

### Required Secrets

Generate and secure these before deployment:

```bash
# JWT Secret (64+ character random string)
openssl rand -hex 64

# Database password (strong random password)
openssl rand -base64 32

# Redis password (if using authentication)
openssl rand -base64 24
```

---

## Environment Configuration

### Production Environment File

Create `.env.production` with production values:

```bash
# ==============================================
# DeeperSensor API - Production Configuration
# ==============================================

# --- Application ---
APP_ENV=production
APP_NAME=deepersensor-api
APP_HOST=0.0.0.0
APP_PORT=8080
APP_PUBLIC_URL=https://api.yourdomain.com

# --- Logging ---
RUST_LOG=info,api=info
LOG_FORMAT=json
REQUEST_ID_HEADER=X-Request-Id

# --- Security / Auth (CHANGE THESE!) ---
JWT_SECRET=<YOUR_64_CHAR_RANDOM_STRING_HERE>
JWT_ISSUER=deepersensor-production
JWT_ACCESS_TTL_SECS=900          # 15 minutes
JWT_REFRESH_TTL_SECS=1209600     # 14 days
ALLOWED_ORIGINS=https://yourdomain.com,https://www.yourdomain.com

# --- Rate Limiting ---
RATE_LIMIT_ENABLED=true
RATE_LIMIT_REQUESTS_PER_MINUTE=60
RATE_LIMIT_BURST=20

# --- Upstream Services ---
OLLAMA_BASE_URL=http://ollama:11434
OLLAMA_DEFAULT_TIMEOUT_MS=30000

# --- Redis ---
REDIS_URL=redis://:YOUR_REDIS_PASSWORD@redis:6379/0

# --- Database (use managed DB in production) ---
DATABASE_URL=postgres://postgres:YOUR_DB_PASSWORD@postgres:5432/deepersensor

# --- HTTP Server ---
SERVER_READ_TIMEOUT_SECS=15
SERVER_WRITE_TIMEOUT_SECS=30
SERVER_IDLE_TIMEOUT_SECS=120
MAX_REQUEST_SIZE_BYTES=1048576

# --- CORS ---
CORS_ALLOW_CREDENTIALS=true
CORS_ALLOW_HEADERS=Authorization,Content-Type,X-Request-Id
CORS_EXPOSE_HEADERS=X-Request-Id
CORS_ALLOW_METHODS=GET,POST,PUT,DELETE,OPTIONS

# --- Proxy Settings ---
TRUSTED_PROXY_IPS=10.0.0.0/8,172.16.0.0/12,192.168.0.0/16
FORCE_HTTPS=true

# --- Monitoring ---
METRICS_ENABLED=true
PROMETHEUS_BIND=0.0.0.0:9500

# --- Build Info ---
GIT_SHA=<set_via_ci_cd>
```

### Secret Management

**DO NOT commit `.env.production` to version control!**

**Options for Production Secrets:**

1. **Environment Variables (Recommended):**
   - AWS Secrets Manager
   - HashiCorp Vault
   - Kubernetes Secrets
   - Docker Swarm Secrets

2. **Encrypted Files:**
   ```bash
   # Encrypt with GPG
   gpg --symmetric --cipher-algo AES256 .env.production
   
   # Decrypt on server
   gpg --decrypt .env.production.gpg > .env.production
   ```

3. **Container Orchestration Secrets:**
   ```bash
   # Docker Swarm
   docker secret create jwt_secret ./jwt_secret.txt
   
   # Kubernetes
   kubectl create secret generic api-secrets \
     --from-literal=jwt-secret="$(cat jwt_secret.txt)"
   ```

---

## Deployment Strategies

### Strategy 1: Docker Compose (Single Server)

**Best for:** Small to medium deployments, MVPs, staging environments

#### Step 1: Clone Repository

```bash
ssh user@your-server
cd /opt
sudo git clone https://github.com/Deepersensor/api.deepersensor.git
cd api.deepersensor
```

#### Step 2: Configure Environment

```bash
# Copy production environment template
sudo cp .env.production.example .env
sudo nano .env  # Edit with your values

# Secure the file
sudo chmod 600 .env
sudo chown root:root .env
```

#### Step 3: Deploy with Docker Compose

```bash
# Pull/build images
sudo docker compose pull
sudo docker compose build --no-cache

# Start services
sudo docker compose up -d

# Verify health
curl http://localhost:8080/health
```

#### Step 4: Configure TLS with Nginx/Caddy

**Option A: Caddy (Easiest)**

```bash
# Install Caddy
sudo apt install -y debian-keyring debian-archive-keyring apt-transport-https
curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/gpg.key' | sudo gpg --dearmor -o /usr/share/keyrings/caddy-stable-archive-keyring.gpg
echo "deb [signed-by=/usr/share/keyrings/caddy-stable-archive-keyring.gpg] https://dl.cloudsmith.io/public/caddy/stable/deb/debian any-version main" | sudo tee /etc/apt/sources.list.d/caddy-stable.list
sudo apt update
sudo apt install caddy

# Configure Caddy
sudo nano /etc/caddy/Caddyfile
```

Caddyfile content:
```caddyfile
api.yourdomain.com {
    reverse_proxy localhost:8181
    encode gzip
    
    log {
        output file /var/log/caddy/access.log
    }
}
```

```bash
sudo systemctl restart caddy
```

**Option B: Let's Encrypt with Certbot + Nginx**

```bash
sudo apt install certbot python3-certbot-nginx
sudo certbot --nginx -d api.yourdomain.com
```

### Strategy 2: Container Registry + Orchestration

**Best for:** Production at scale, high availability

#### Using GitHub Container Registry

```bash
# Login to GHCR
echo $GITHUB_TOKEN | docker login ghcr.io -u USERNAME --password-stdin

# Pull production images
docker pull ghcr.io/deepersensor/api.deepersensor/api:latest
docker pull ghcr.io/deepersensor/api.deepersensor/nginx:latest

# Run with production compose file
docker compose -f docker-compose.prod.yml up -d
```

#### Kubernetes Deployment

See `k8s/` directory for manifests (coming soon).

```bash
# Apply configurations
kubectl apply -f k8s/namespace.yml
kubectl apply -f k8s/secrets.yml
kubectl apply -f k8s/configmap.yml
kubectl apply -f k8s/postgres.yml
kubectl apply -f k8s/redis.yml
kubectl apply -f k8s/api-deployment.yml
kubectl apply -f k8s/ingress.yml
```

---

## Database Management

### Initial Setup

```bash
# Database is automatically migrated on API startup
# Migrations are in ./migrations/ and run via sqlx

# Verify migration status
docker compose exec api /app/api --check-migrations  # (future feature)
```

### Manual Migration

```bash
# Install sqlx-cli
cargo install sqlx-cli --no-default-features --features postgres

# Run migrations manually
DATABASE_URL=postgres://user:pass@host/db sqlx migrate run
```

### Backup Strategy

#### Automated Daily Backups

```bash
# Create backup script
sudo nano /usr/local/bin/backup-deepersensor-db.sh
```

```bash
#!/bin/bash
set -e

BACKUP_DIR="/var/backups/deepersensor"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
DB_NAME="deepersensor"
DB_USER="postgres"
DB_HOST="localhost"

mkdir -p "$BACKUP_DIR"

# Backup with pg_dump
PGPASSWORD="$DB_PASSWORD" pg_dump \
  -h "$DB_HOST" \
  -U "$DB_USER" \
  -d "$DB_NAME" \
  -F c \
  -f "$BACKUP_DIR/backup_${TIMESTAMP}.dump"

# Compress
gzip "$BACKUP_DIR/backup_${TIMESTAMP}.dump"

# Rotate old backups (keep 30 days)
find "$BACKUP_DIR" -name "backup_*.dump.gz" -mtime +30 -delete

echo "Backup completed: backup_${TIMESTAMP}.dump.gz"
```

```bash
sudo chmod +x /usr/local/bin/backup-deepersensor-db.sh

# Add to crontab (daily at 2 AM)
sudo crontab -e
0 2 * * * /usr/local/bin/backup-deepersensor-db.sh >> /var/log/db-backup.log 2>&1
```

#### Restore from Backup

```bash
# Stop API to prevent writes
docker compose stop api

# Restore database
gunzip -c /var/backups/deepersensor/backup_20250929_020000.dump.gz | \
  PGPASSWORD="$DB_PASSWORD" pg_restore \
    -h localhost \
    -U postgres \
    -d deepersensor \
    --clean

# Restart API
docker compose start api
```

### Database Maintenance

```bash
# Vacuum and analyze (weekly)
docker compose exec postgres psql -U postgres -d deepersensor -c "VACUUM ANALYZE;"

# Check database size
docker compose exec postgres psql -U postgres -d deepersensor -c "
  SELECT pg_size_pretty(pg_database_size('deepersensor'));"
```

---

## Security Hardening

### TLS Configuration

**Minimum TLS version:** TLS 1.2  
**Recommended:** TLS 1.3 only

```nginx
# In nginx or load balancer
ssl_protocols TLSv1.3 TLSv1.2;
ssl_ciphers 'ECDHE-ECDSA-AES128-GCM-SHA256:ECDHE-RSA-AES128-GCM-SHA256...';
ssl_prefer_server_ciphers off;
```

### Firewall Rules

```bash
# UFW (Ubuntu)
sudo ufw default deny incoming
sudo ufw default allow outgoing
sudo ufw allow 22/tcp    # SSH
sudo ufw allow 80/tcp    # HTTP
sudo ufw allow 443/tcp   # HTTPS
sudo ufw enable

# Allow only from specific IPs for management
sudo ufw allow from YOUR_ADMIN_IP to any port 22
```

### Container Security

```yaml
# docker-compose.prod.yml security settings
services:
  api:
    security_opt:
      - no-new-privileges:true
    cap_drop:
      - ALL
    cap_add:
      - NET_BIND_SERVICE
    read_only: true
    tmpfs:
      - /tmp
```

### Secrets Rotation

**Rotate JWT secrets every 90 days:**

```bash
# Generate new secret
NEW_SECRET=$(openssl rand -hex 64)

# Update .env
sed -i "s/JWT_SECRET=.*/JWT_SECRET=$NEW_SECRET/" .env

# Rolling restart
docker compose up -d --no-deps --force-recreate api
```

---

## Monitoring & Observability

### Health Checks

```bash
# Application health
curl https://api.yourdomain.com/health

# Detailed health with dependencies (future)
curl https://api.yourdomain.com/readiness
```

### Prometheus Metrics

```yaml
# prometheus.yml
scrape_configs:
  - job_name: 'deepersensor-api'
    static_configs:
      - targets: ['api:9500']
    scrape_interval: 15s
```

### Log Aggregation

**Using Loki:**

```yaml
# docker-compose.monitoring.yml
services:
  loki:
    image: grafana/loki:latest
    ports:
      - "3100:3100"
    
  promtail:
    image: grafana/promtail:latest
    volumes:
      - /var/lib/docker/containers:/var/lib/docker/containers:ro
      - ./promtail-config.yml:/etc/promtail/config.yml
```

### Alerting Rules

Set up alerts for:
- API response time > 1s (p95)
- Error rate > 1%
- Database connection failures
- Disk usage > 80%
- Memory usage > 90%
- SSL certificate expiring < 30 days

---

## Backup & Recovery

### Disaster Recovery Plan

1. **Database Backups:** Automated daily + manual before major changes
2. **Configuration Backups:** `.env`, nginx configs, docker-compose files
3. **Ollama Models:** Backup model files if using custom-trained models
4. **Recovery Time Objective (RTO):** < 1 hour
5. **Recovery Point Objective (RPO):** < 24 hours

### Recovery Procedure

```bash
# 1. Provision new server
# 2. Install dependencies (Docker, etc.)
# 3. Clone repository
# 4. Restore .env from secure storage
# 5. Restore database
gunzip -c backup.dump.gz | pg_restore -d deepersensor

# 6. Start services
docker compose up -d

# 7. Verify health
curl http://localhost/health
```

---

## Scaling & Performance

### Horizontal Scaling

```yaml
# docker-compose.scale.yml
services:
  api:
    deploy:
      replicas: 3
    
  nginx:
    depends_on:
      - api
    # Load balance across replicas
```

### Vertical Scaling

```yaml
services:
  api:
    deploy:
      resources:
        limits:
          cpus: '2'
          memory: 4G
        reservations:
          cpus: '1'
          memory: 2G
```

### Database Connection Pooling

Already configured in `crates/core/src/config.rs`:

```rust
// Adjust in AppConfig
max_connections: 20,
min_connections: 5,
```

### Caching Strategy

- Redis for rate limiting (in-memory fallback)
- HTTP caching headers for model lists
- Database query result caching (future enhancement)

---

## Troubleshooting

### Common Issues

#### API Not Starting

```bash
# Check logs
docker compose logs api

# Common causes:
# - Missing JWT_SECRET
# - Database connection failure
# - Port already in use
```

#### Database Connection Errors

```bash
# Verify database is up
docker compose exec postgres pg_isready

# Check credentials
docker compose exec postgres psql -U postgres -d deepersensor -c "SELECT 1;"

# Check network connectivity
docker compose exec api ping postgres
```

#### High Memory Usage

```bash
# Check container stats
docker stats

# Reduce connection pool size in config
# Restart with memory limits
docker compose up -d --force-recreate
```

#### Rate Limiting Too Aggressive

```bash
# Adjust in .env
RATE_LIMIT_REQUESTS_PER_MINUTE=120
RATE_LIMIT_BURST=40

docker compose restart api
```

### Debug Mode

```bash
# Enable debug logs
RUST_LOG=debug,api=trace docker compose up api

# Or edit .env
RUST_LOG=debug,api=trace
```

---

## Rollback Procedure

```bash
# 1. Stop current version
docker compose stop api

# 2. Pull previous image tag
docker pull ghcr.io/deepersensor/api:v1.2.3

# 3. Update docker-compose to use specific tag
# Edit docker-compose.yml: image: ghcr.io/deepersensor/api:v1.2.3

# 4. Restore database if migrations changed
gunzip -c backup_before_update.dump.gz | pg_restore -d deepersensor

# 5. Start previous version
docker compose up -d api

# 6. Verify
curl https://api.yourdomain.com/health
```

---

## Maintenance Windows

Recommended schedule:
- **Security patches:** As needed (off-hours)
- **Minor updates:** Bi-weekly (Sundays 2-4 AM)
- **Major updates:** Monthly (planned, with announcement)
- **Database maintenance:** Weekly (Sundays 3 AM)

---

## Support & Resources

- Documentation: https://github.com/Deepersensor/api.deepersensor
- Issues: https://github.com/Deepersensor/api.deepersensor/issues
- Security: security@deepersensor.com
- Status Page: (configure with UptimeRobot or similar)

---

**Last Review:** September 2025  
**Next Review:** December 2025
