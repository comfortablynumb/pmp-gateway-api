# Integration Tests

This directory contains comprehensive integration tests for the PMP Gateway API using [Hurl](https://hurl.dev/).

## Test Coverage

The test suite covers all major features of the gateway:

1. **01-healthcheck.hurl** - Basic health check endpoint
2. **02-admin-api.hurl** - Administrative API endpoints
3. **03-request-id.hurl** - Request ID generation and propagation
4. **04-cors.hurl** - CORS headers and preflight requests
5. **05-rate-limiting.hurl** - Rate limiting functionality
6. **06-caching.hurl** - Response caching with cache headers
7. **07-deduplication.hurl** - Request deduplication and idempotency
8. **08-security.hurl** - Security headers and IP filtering
9. **09-metrics.hurl** - Prometheus metrics endpoint
10. **10-circuit-breaker.hurl** - Circuit breaker pattern
11. **11-traffic-mirror.hurl** - Traffic mirroring to test backends
12. **12-traffic-split.hurl** - A/B testing and canary deployments
13. **13-tracing.hurl** - Distributed tracing with OpenTelemetry
14. **14-websocket.hurl** - WebSocket upgrade and proxying
15. **15-compression.hurl** - Response compression (gzip/brotli)
16. **16-load-balancing.hurl** - Load balancing strategies
17. **17-transformation.hurl** - Request/response transformations
18. **18-environment-config.hurl** - Environment-specific configurations
19. **19-integration-flow.hurl** - End-to-end integration flows
20. **20-error-handling.hurl** - Error responses and edge cases

## Running Tests

### Using Docker Compose

Run all integration tests:

```bash
docker-compose --profile integration-tests up --abort-on-container-exit
```

Run tests and rebuild the app:

```bash
docker-compose --profile integration-tests up --build --abort-on-container-exit
```

### Using Hurl Directly

If you have Hurl installed locally and the app running:

```bash
# Run all tests
hurl --test --glob "resources/integration-tests/**/*.hurl" \
     --variables-file resources/integration-tests/variables.env

# Run a specific test file
hurl --test resources/integration-tests/01-healthcheck.hurl

# Run with verbose output
hurl --test --verbose resources/integration-tests/02-admin-api.hurl

# Run and generate HTML report
hurl --test --report-html test-report --glob "resources/integration-tests/**/*.hurl"
```

## Test Variables

The `variables.env` file contains configuration variables:

- `base_url` - Base URL of the gateway (default: http://app:8080)
- `admin_base_url` - Admin API base URL (default: http://app:8080/admin)
- `backend_url` - Mock backend URL (default: http://mock-backend:1080)

Override variables:

```bash
hurl --test --variable base_url=http://localhost:8080 \
     resources/integration-tests/01-healthcheck.hurl
```

## Test Requirements

### Dependencies

The integration tests expect the following services to be running:

- **PMP Gateway** (port 8080)
- **PostgreSQL** (port 5432)
- **MySQL** (port 3306)
- **MongoDB** (port 27017)
- **Redis** (port 6379)
- **Mock Backend** (port 1080) - MockServer for backend simulation

### Configuration

Ensure the gateway is configured with routes and clients that match the test expectations. Example routes needed:

- `/health` - Health check endpoint
- `/admin/*` - Admin API endpoints
- `/metrics` - Prometheus metrics
- `/api/*` - Test API routes
- `/ws` - WebSocket endpoint

## Writing New Tests

### Hurl Test Format

```hurl
# Test description
GET {{base_url}}/endpoint
Header-Name: header-value
Content-Type: application/json
{
  "key": "value"
}
HTTP 200
[Asserts]
jsonpath "$.field" == "expected"
header "Header-Name" exists
body contains "text"
```

### Best Practices

1. **Use descriptive comments** - Explain what each test verifies
2. **Test one feature per file** - Keep tests focused and organized
3. **Use assertions liberally** - Verify headers, status, and body
4. **Capture values** - Use `[Captures]` for multi-step tests
5. **Handle optional features** - Use `OR` assertions when features are optional

### Example Multi-Step Test

```hurl
# Step 1: Create resource
POST {{base_url}}/api/resources
Content-Type: application/json
{
  "name": "test"
}
HTTP 201
[Captures]
resource_id: jsonpath "$.id"

# Step 2: Retrieve created resource
GET {{base_url}}/api/resources/{{resource_id}}
HTTP 200
[Asserts]
jsonpath "$.name" == "test"
```

## Continuous Integration

Add to your CI/CD pipeline:

```yaml
# GitHub Actions example
- name: Run Integration Tests
  run: |
    docker-compose --profile integration-tests up --build --abort-on-container-exit
    exit_code=$?
    docker-compose down -v
    exit $exit_code
```

## Troubleshooting

### Tests Failing

1. **Check service health**:
   ```bash
   docker-compose ps
   docker-compose logs app
   ```

2. **Verify configuration**:
   ```bash
   docker-compose exec app cat /app/config.yaml
   ```

3. **Check backend connectivity**:
   ```bash
   docker-compose exec app curl -v http://mock-backend:1080/health
   ```

### Debugging Tests

Run with verbose output:

```bash
hurl --test --verbose --very-verbose resources/integration-tests/01-healthcheck.hurl
```

Print response bodies:

```bash
hurl --test --include resources/integration-tests/02-admin-api.hurl
```

## Resources

- [Hurl Documentation](https://hurl.dev/docs/manual.html)
- [Hurl Tutorial](https://hurl.dev/docs/tutorial/getting-started.html)
- [Assertion Reference](https://hurl.dev/docs/asserting-response.html)
