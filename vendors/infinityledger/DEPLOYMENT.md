# MEF CI/CD and Deployment Guide

This guide covers the Continuous Integration, Continuous Deployment, testing, monitoring, and deployment automation for the MEF (Multi-dimensional Embedding Framework) project.

## Table of Contents

- [CI/CD Pipeline](#cicd-pipeline)
- [Testing](#testing)
- [Performance Benchmarking](#performance-benchmarking)
- [API Documentation](#api-documentation)
- [Load Testing](#load-testing)
- [Monitoring](#monitoring)
- [Deployment](#deployment)
- [Production Configuration](#production-configuration)

## CI/CD Pipeline

The project uses GitHub Actions for continuous integration and deployment.

### Workflows

#### Rust CI/CD (`.github/workflows/rust-ci.yml`)

The main CI/CD pipeline includes:

1. **Lint and Format**
   - Code formatting checks (`cargo fmt`)
   - Linting with Clippy (`cargo clippy`)

2. **Build and Test**
   - Build all workspace crates
   - Run unit tests
   - Run integration tests
   - Generate documentation

3. **Integration Tests with Services**
   - Start API server
   - Run integration tests against live services

4. **Performance Benchmarks**
   - Run performance benchmarks
   - Upload benchmark results

5. **Security Audit**
   - Check for security vulnerabilities with `cargo audit`

6. **Build Release Artifacts**
   - Build optimized release binaries
   - Upload artifacts

7. **Docker Build and Push**
   - Build Docker images
   - Push to GitHub Container Registry

### Running CI Locally

```bash
# Run linting
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings

# Run tests
cargo test --workspace

# Build release
cargo build --release
```

## Testing

### Unit Tests

Unit tests are located in each crate's `src` directory:

```bash
# Run all unit tests
cargo test --workspace --lib

# Run tests for specific crate
cargo test -p mef-spiral
cargo test -p mef-ledger
```

### Integration Tests

Integration tests are in the `tests/` directory:

```bash
# Run all integration tests
cargo test --workspace --test '*'

# Run specific integration test
cargo test --test integration_core
cargo test --test integration_api
```

**Note:** API integration tests require the API server to be running:

```bash
# Terminal 1: Start API server
cargo run --package mef-api --bin mef-api

# Terminal 2: Run API integration tests
cargo test --test integration_api -- --ignored
```

### Test Coverage

Generate test coverage with `cargo-tarpaulin`:

```bash
# Install tarpaulin
cargo install cargo-tarpaulin

# Generate coverage report
cargo tarpaulin --out Html --output-dir coverage
```

## Performance Benchmarking

### Baseline Benchmarks

Performance benchmarks are defined in `benches/performance_baseline.rs`:

```bash
# Run benchmarks
cargo bench

# Run specific benchmark
cargo bench spiral_snapshot
cargo bench ledger
```

### Benchmark Reports

Benchmark results are saved in `target/criterion/` and include:

- HTML reports with performance graphs
- Statistical analysis (p50, p95, p99)
- Performance comparisons over time

### Creating a Performance Baseline

```bash
# Run benchmarks and save baseline
cargo bench --bench performance_baseline > benchmark-results/baseline.txt

# Compare against baseline
cargo bench --bench performance_baseline
```

## API Documentation

### OpenAPI/Swagger

The API is documented using OpenAPI 3.0 in `openapi.yaml`.

**View Documentation:**

1. Using Swagger UI:
   ```bash
   # Install swagger-ui
   npm install -g swagger-ui-watcher
   
   # Serve documentation
   swagger-ui-watcher openapi.yaml
   ```

2. Using Redoc:
   ```bash
   # Install redoc-cli
   npm install -g redoc-cli
   
   # Generate HTML
   redoc-cli bundle openapi.yaml -o docs/api.html
   ```

3. Online viewers:
   - Upload `openapi.yaml` to https://editor.swagger.io
   - Use Stoplight: https://stoplight.io

### Rust Documentation

Generate Rust API documentation:

```bash
# Generate and open documentation
cargo doc --no-deps --open

# Generate documentation for all crates
cargo doc --workspace --open
```

## Load Testing

Load tests are implemented using k6 in `load-tests/api-load-test.js`.

### Running Load Tests

**Prerequisites:**
```bash
# Install k6
# On macOS
brew install k6

# On Linux
sudo gpg -k
sudo gpg --no-default-keyring --keyring /usr/share/keyrings/k6-archive-keyring.gpg --keyserver hkp://keyserver.ubuntu.com:80 --recv-keys C5AD17C747E3415A3642D57D77C6C491D6AC1D69
echo "deb [signed-by=/usr/share/keyrings/k6-archive-keyring.gpg] https://dl.k6.io/deb stable main" | sudo tee /etc/apt/sources.list.d/k6.list
sudo apt-get update
sudo apt-get install k6
```

**Run Tests:**

```bash
# Start API server
cargo run --package mef-api --bin mef-api

# Run load test (in another terminal)
k6 run --vus 10 --duration 30s load-tests/api-load-test.js

# Run with custom settings
k6 run --vus 50 --duration 2m load-tests/api-load-test.js

# Run with custom endpoint
API_BASE_URL=http://your-server:8080 k6 run load-tests/api-load-test.js
```

**Using Docker:**

```bash
# Run with docker-compose
docker-compose -f docker-compose.rust.yml --profile loadtest up k6
```

### Load Test Scenarios

The load test includes:

- **70%** Search operations
- **20%** Upsert operations
- **10%** Snapshot operations
- **Random** Health checks

**Performance Thresholds:**

- 95th percentile latency < 500ms
- 99th percentile latency < 1000ms
- Error rate < 10%

## Monitoring

### Prometheus Metrics

The API server exposes Prometheus metrics at `/metrics`:

```bash
# View metrics
curl http://localhost:9090/metrics
```

**Available Metrics:**

- Request count and latency
- Database operation metrics
- Index operation metrics
- Memory usage
- Error rates

### Grafana Dashboards

Start Grafana with monitoring:

```bash
docker-compose -f docker-compose.rust.yml --profile monitoring up
```

**Access:**

- Grafana: http://localhost:3000 (admin/admin)
- Prometheus: http://localhost:9091

**Dashboards:**

Pre-configured dashboards are available in `infinity-ledger-main/monitoring/grafana/dashboards/`.

### Health Monitoring

Health check endpoint:

```bash
curl http://localhost:8080/healthz
```

Response:
```json
{
  "status": "ok",
  "version": "1.0.0",
  "timestamp": "2025-10-15T18:00:00Z"
}
```

## Deployment

### Deployment Script

Use the automated deployment script:

```bash
# Deploy to development
./deploy/deploy.sh dev

# Deploy to staging
./deploy/deploy.sh staging v1.0.0

# Deploy to production
./deploy/deploy.sh production v1.0.0

# Check deployment status
./deploy/deploy.sh status production

# Rollback deployment
./deploy/deploy.sh rollback production
```

### Manual Deployment

#### Development

```bash
# Build and run locally
cargo run --package mef-api --bin mef-api
```

#### Docker

```bash
# Build Docker image
docker build -t mef-api:latest .

# Run container
docker run -p 8080:8080 -p 9090:9090 mef-api:latest
```

#### Docker Compose

```bash
# Development
docker-compose -f docker-compose.rust.yml up

# Production with monitoring
docker-compose -f docker-compose.rust.yml --profile production --profile monitoring up -d
```

### Kubernetes (Optional)

Deploy to Kubernetes:

```bash
# Create namespace
kubectl create namespace mef

# Apply configurations
kubectl apply -f k8s/

# Check deployment
kubectl get pods -n mef
```

## Production Configuration

### Environment Variables

Production configuration is in `config/production.env`. Key settings:

```bash
# Server
BIND_HOST=0.0.0.0
PORT=8080
ENVIRONMENT=production

# Security
AUTH_TOKEN_REQUIRED=true
# Set MEF_API_TOKEN via Docker secrets

# Logging
LOG_LEVEL=info
LOG_FORMAT=json

# Metrics
ENABLE_METRICS=true
METRICS_PORT=9090

# Performance
WORKER_THREADS=0  # Auto-detect
REQUEST_TIMEOUT=30
MAX_CONNECTIONS=1000
```

### Resource Limits

**Docker Compose:**

```yaml
deploy:
  resources:
    limits:
      cpus: "2.0"
      memory: "2G"
    reservations:
      cpus: "0.5"
      memory: "512M"
```

**Environment Variables:**

```bash
API_CPU_LIMIT=2.0
API_MEMORY_LIMIT=2G
```

### Security Best Practices

1. **Use Docker Secrets:**
   ```bash
   echo "your-secret-token" | docker secret create mef_api_token -
   ```

2. **Enable HTTPS:**
   - Use a reverse proxy (nginx, Traefik)
   - Configure TLS certificates

3. **Rate Limiting:**
   ```bash
   ENABLE_RATE_LIMIT=true
   RATE_LIMIT_RPS=100
   ```

4. **Authentication:**
   ```bash
   AUTH_TOKEN_REQUIRED=true
   MEF_API_TOKEN=<secret>
   ```

### Backup and Recovery

**Enable Auto-Backup:**

```bash
ENABLE_AUTO_BACKUP=true
BACKUP_INTERVAL_HOURS=24
BACKUP_RETENTION_DAYS=30
```

**Manual Backup:**

```bash
# Backup data directory
docker exec mef-api tar -czf /tmp/backup.tar.gz /data

# Copy backup from container
docker cp mef-api:/tmp/backup.tar.gz ./backups/backup-$(date +%Y%m%d).tar.gz
```

## Troubleshooting

### Common Issues

1. **Build Fails:**
   ```bash
   # Clean build
   cargo clean
   cargo build
   ```

2. **Tests Fail:**
   ```bash
   # Check logs
   RUST_LOG=debug cargo test
   ```

3. **Container Won't Start:**
   ```bash
   # Check logs
   docker-compose logs mef-api
   
   # Check health
   docker-compose ps
   ```

4. **Performance Issues:**
   ```bash
   # Check metrics
   curl http://localhost:9090/metrics
   
   # Check resource usage
   docker stats mef-api
   ```

### Getting Help

- GitHub Issues: https://github.com/LashSesh/infinityledger/issues
- Documentation: https://github.com/LashSesh/infinityledger
- API Docs: http://localhost:8080/docs (when running)

## License

MIT License - see LICENSE file for details.
