# Docker Deployment Guide

This guide explains how to build and run the PMP Gateway API using Docker and Docker Compose.

## Quick Start

### Development Mode

Run the application with all dependencies:

```bash
docker-compose --profile app up --build
```

The gateway will be available at `http://localhost:8080`

### Integration Tests

Run the full integration test suite:

```bash
docker-compose --profile integration-tests up --build --abort-on-container-exit
```

This will:
1. Start all required services (PostgreSQL, MySQL, MongoDB, Redis, Mock Backend)
2. Build and start the PMP Gateway
3. Wait for all services to be healthy
4. Run the Hurl integration tests
5. Exit with the test results

## Services

### PMP Gateway (app)

The main API gateway application.

- **Port**: 8080
- **Health**: `http://localhost:8080/health`
- **Metrics**: `http://localhost:8080/metrics`
- **Admin API**: `http://localhost:8080/admin/*`

### PostgreSQL (postgres)

PostgreSQL database for testing.

- **Port**: 5432
- **Database**: testdb
- **User**: testuser
- **Password**: testpass

### MySQL (mysql)

MySQL database for testing.

- **Port**: 3306
- **Database**: testdb
- **User**: testuser
- **Password**: testpass

### MongoDB (mongodb)

MongoDB database for testing.

- **Port**: 27017
- **User**: testuser
- **Password**: testpass

### Redis (redis)

Redis cache for testing.

- **Port**: 6379

### Mock Backend (mock-backend)

MockServer for simulating backend services.

- **Port**: 1080
- **Dashboard**: `http://localhost:1080/mockserver/dashboard`

## Building the Docker Image

### Build Locally

```bash
docker build -t pmp-gateway-api:latest .
```

### Multi-stage Build

The Dockerfile uses a multi-stage build:

1. **Builder stage**: Compiles the Rust application
2. **Runtime stage**: Creates a minimal runtime image with only the binary

### Build Arguments

Customize the build with build arguments:

```bash
docker build \
  --build-arg RUST_VERSION=1.75 \
  -t pmp-gateway-api:latest \
  .
```

## Running the Application

### Standalone Container

Run just the gateway (requires external services):

```bash
docker run -p 8080:8080 \
  -v $(pwd)/config.yaml:/app/config.yaml:ro \
  -e RUST_LOG=info \
  pmp-gateway-api:latest
```

### With Docker Compose

#### Start all services:

```bash
docker-compose up -d
```

#### View logs:

```bash
docker-compose logs -f app
```

#### Stop services:

```bash
docker-compose down
```

#### Clean up volumes:

```bash
docker-compose down -v
```

## Environment Variables

Configure the gateway using environment variables:

| Variable | Description | Default |
|----------|-------------|---------|
| `RUST_LOG` | Log level (trace, debug, info, warn, error) | `info` |
| `CONFIG_PATH` | Path to configuration file | `/app/config.yaml` |
| `DATABASE_URL` | PostgreSQL connection string | - |
| `MYSQL_URL` | MySQL connection string | - |
| `MONGODB_URL` | MongoDB connection string | - |
| `REDIS_URL` | Redis connection string | - |
| `ENV` | Environment name (dev, prod, etc.) | - |

Example:

```bash
docker run -p 8080:8080 \
  -e RUST_LOG=debug \
  -e ENV=prod \
  -e DATABASE_URL=postgres://user:pass@db:5432/mydb \
  pmp-gateway-api:latest
```

## Configuration

### Volume Mounts

Mount custom configuration:

```bash
docker run -p 8080:8080 \
  -v /path/to/config.yaml:/app/config.yaml:ro \
  -v /path/to/config.prod.yaml:/app/config.prod.yaml:ro \
  pmp-gateway-api:latest
```

### Environment-Specific Configs

Use environment-specific configuration files:

```bash
docker run -p 8080:8080 \
  -v $(pwd)/config.yaml:/app/config.yaml:ro \
  -v $(pwd)/config.prod.yaml:/app/config.prod.yaml:ro \
  -e ENV=prod \
  pmp-gateway-api:latest
```

## Health Checks

The Docker image includes health checks:

```bash
# Check container health
docker inspect --format='{{.State.Health.Status}}' <container-id>

# View health check logs
docker inspect --format='{{range .State.Health.Log}}{{.Output}}{{end}}' <container-id>
```

Docker Compose health checks:

- **Interval**: 10s
- **Timeout**: 5s
- **Retries**: 3
- **Start period**: 10s

## Production Deployment

### Recommended Settings

```yaml
services:
  app:
    build: .
    restart: unless-stopped
    ports:
      - "8080:8080"
    environment:
      RUST_LOG: info
      ENV: prod
    volumes:
      - ./config.yaml:/app/config.yaml:ro
      - ./config.prod.yaml:/app/config.prod.yaml:ro
    deploy:
      resources:
        limits:
          cpus: '2'
          memory: 2G
        reservations:
          cpus: '1'
          memory: 512M
```

### Security Best Practices

1. **Use specific image tags**: Don't use `:latest` in production
2. **Run as non-root**: Add a user in the Dockerfile
3. **Read-only filesystem**: Mount configs as read-only (`:ro`)
4. **Secrets management**: Use Docker secrets or environment variables
5. **Network isolation**: Use Docker networks for service communication

### Scaling

Scale the gateway horizontally:

```bash
docker-compose up -d --scale app=3
```

Use a load balancer (nginx, traefik) in front of multiple instances.

## Troubleshooting

### Container Won't Start

Check logs:

```bash
docker-compose logs app
```

Verify configuration:

```bash
docker-compose exec app cat /app/config.yaml
```

### Health Check Failing

Test health endpoint manually:

```bash
docker-compose exec app wget -O- http://localhost:8080/health
```

### Database Connection Issues

Check service connectivity:

```bash
docker-compose exec app ping postgres
docker-compose exec app nc -zv postgres 5432
```

### Performance Issues

Monitor resource usage:

```bash
docker stats
```

Increase resource limits in docker-compose.yaml.

## CI/CD Integration

### GitHub Actions

```yaml
name: Docker Build and Test

on: [push]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Build Docker image
        run: docker-compose build

      - name: Run integration tests
        run: |
          docker-compose --profile integration-tests up --abort-on-container-exit
          docker-compose down -v
```

### GitLab CI

```yaml
test:
  image: docker:latest
  services:
    - docker:dind
  script:
    - docker-compose --profile integration-tests up --build --abort-on-container-exit
    - docker-compose down -v
```

## Resources

- [Docker Documentation](https://docs.docker.com/)
- [Docker Compose Documentation](https://docs.docker.com/compose/)
- [Rust Docker Images](https://hub.docker.com/_/rust)
- [Debian Slim Images](https://hub.docker.com/_/debian)
