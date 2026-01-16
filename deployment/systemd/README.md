# CAGE Systemd Service

## Installation

```bash
# 1. Build release binary
cd orchestrator
cargo build --release

# 2. Install binary
sudo mkdir -p /opt/cage/bin
sudo cp target/release/cage-orchestrator /opt/cage/bin/
sudo chmod +x /opt/cage/bin/cage-orchestrator

# 3. Create cage user
sudo useradd -r -s /bin/false cage
sudo mkdir -p /var/lib/cage /var/log/cage
sudo chown cage:cage /var/lib/cage /var/log/cage

# 4. Install config
sudo mkdir -p /etc/cage
sudo cp ../config/cage.yaml /etc/cage/config.yaml
sudo cp ../sandbox/seccomp.json /etc/cage/seccomp.json

# 5. Edit config for production
sudo vim /etc/cage/config.yaml
# Change jwt_secret, admin_token, paths

# 6. Install service
sudo cp cage-orchestrator.service /etc/systemd/system/
sudo systemctl daemon-reload

# 7. Start service
sudo systemctl enable cage-orchestrator
sudo systemctl start cage-orchestrator

# 8. Check status
sudo systemctl status cage-orchestrator
sudo journalctl -u cage-orchestrator -f
```

## Configuration

### Environment Variables

Create `/etc/cage/environment`:

```bash
CAGE__DATA_DIR=/var/lib/cage
CAGE__LOG_LEVEL=info
CAGE__SECURITY__JWT_SECRET=your-secret-here
CAGE__SECURITY__ADMIN_TOKEN=your-admin-token
```

### Service Management

```bash
# Start
sudo systemctl start cage-orchestrator

# Stop
sudo systemctl stop cage-orchestrator

# Restart
sudo systemctl restart cage-orchestrator

# Status
sudo systemctl status cage-orchestrator

# Logs
sudo journalctl -u cage-orchestrator -f

# Enable auto-start
sudo systemctl enable cage-orchestrator

# Disable auto-start
sudo systemctl disable cage-orchestrator
```

## Troubleshooting

### Service fails to start

```bash
# Check logs
sudo journalctl -u cage-orchestrator -n 50

# Check permissions
ls -la /var/lib/cage
ls -la /var/log/cage

# Check binary
/opt/cage/bin/cage-orchestrator --version

# Test config
sudo -u cage /opt/cage/bin/cage-orchestrator
```

### Permission denied errors

```bash
# Ensure cage user owns data directory
sudo chown -R cage:cage /var/lib/cage

# Check Podman access
sudo -u cage podman ps
```

### Port already in use

```bash
# Check what's using port 8080
sudo lsof -i :8080

# Change port in config
sudo vim /etc/cage/config.yaml
# Set port: 8081
sudo systemctl restart cage-orchestrator
```

## Security Hardening

Service includes:
- `NoNewPrivileges=true` - Cannot gain privileges
- `PrivateTmp=true` - Isolated /tmp
- `ProtectSystem=strict` - Read-only system directories
- `ProtectHome=true` - No access to user homes
- `ReadWritePaths` - Only /var/lib/cage and /var/log/cage writable

## Monitoring

```bash
# Real-time logs
sudo journalctl -u cage-orchestrator -f

# System resource usage
sudo systemctl status cage-orchestrator

# Detailed status
sudo systemctl show cage-orchestrator
```
