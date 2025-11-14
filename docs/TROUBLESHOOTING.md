# Troubleshooting Guide

Common issues and their solutions for PMP Gateway API.

## Table of Contents

- [Installation Issues](#installation-issues)
- [Configuration Errors](#configuration-errors)
- [Runtime Errors](#runtime-errors)
- [Performance Issues](#performance-issues)
- [Database Connectivity](#database-connectivity)
- [Debugging Tips](#debugging-tips)

---

## Installation Issues

### Cargo Build Fails

**Error:**
```
error: linker `cc` not found
```

**Solution:**
```bash
# Ubuntu/Debian
sudo apt-get install build-essential

# CentOS/RHEL
sudo yum groupinstall "Development Tools"

# macOS
xcode-select --install
```

### SQLx Compilation Issues

**Error:**
```
error: failed to compile `sqlx`
```

**Solution:**
```bash
# Install required system libraries
# Ubuntu/Debian
sudo apt-get install libssl-dev pkg-config

# macOS
brew install openssl pkg-config
```

### MongoDB Driver Issues

**Error:**
```
error: failed to run custom build command for `mongodb`
```

**Solution:**
```bash
# Ensure you have OpenSSL installed
# Ubuntu/Debian
sudo apt-get install libssl-dev

# Set environment variable if needed
export OPENSSL_DIR=/usr/local/opt/openssl
```

---

## Configuration Errors

### Unknown Client ID

**Error:**
```
Route /users/:id references unknown client_id: api_client
```

**Cause:**
Subrequest references a client that doesn't exist in the `clients` section.

**Solution:**
```yaml
# Make sure client is defined
clients:
  api_client:  # Must match the client_id in subrequests
    type: http
    base_url: "..."

routes:
  - method: GET
    path: /users/:id
    subrequests:
      - client_id: api_client  # Must match above
        type: http
        uri: /users/1
```

### Circular Dependency

**Error:**
```
Circular dependency detected in subrequests
```

**Cause:**
Subrequests have circular dependencies (A depends on B, B depends on A).

**Solution:**
```yaml
# Bad - circular dependency
subrequests:
  - name: a
    depends_on: [b]
  - name: b
    depends_on: [a]  # Circular!

# Good - linear dependency
subrequests:
  - name: a
    # No dependencies
  - name: b
    depends_on: [a]
  - name: c
    depends_on: [b]
```

### Invalid YAML Syntax

**Error:**
```
failed to deserialize config: invalid type
```

**Cause:**
YAML syntax error or incorrect data types.

**Solution:**
```yaml
# Bad - missing quotes
connection_string: postgres://user:pass@host/db

# Good - quoted string
connection_string: "postgres://user:pass@host/db"

# Bad - wrong type
max_connections: "10"  # String instead of number

# Good - correct type
max_connections: 10
```

**Validation Tools:**
```bash
# Check YAML syntax
yamllint config.yaml

# Validate with application
cargo run -- --validate-config
```

### Interpolation Syntax Error

**Error:**
```
Interpolation failed: invalid expression
```

**Cause:**
Incorrect interpolation syntax.

**Solution:**
```yaml
# Bad - missing quotes in header name
headers:
  Auth: "${request.headers[authorization]}"

# Good - proper quotes
headers:
  Auth: "${request.headers[\"authorization\"]}"

# Bad - wrong accessor
value: "${request.path[id]}"

# Good - dot notation for path params
value: "${request.path.id}"
```

---

## Runtime Errors

### Client Not Found

**Error:**
```
Client not found: redis_cache
```

**Cause:**
Client initialization failed or wrong client_id.

**Debug:**
```bash
# Check logs for initialization errors
RUST_LOG=debug cargo run
```

Look for:
```
Creating Redis client
ERROR: Failed to connect to redis://localhost:6379
```

**Solutions:**
1. Verify service is running
2. Check connection string
3. Verify network connectivity
4. Check firewall rules

### Subrequest Failed

**Error:**
```
Subrequest failed: connection refused
```

**Cause:**
Backend service unavailable.

**Debug:**
```bash
# Test connection directly
curl http://api.example.com/endpoint

# For databases
psql postgres://user:pass@host/db -c "SELECT 1"
mongo mongodb://localhost:27017
redis-cli -h localhost -p 6379 ping
```

**Solutions:**
1. Verify service is running
2. Check connection details
3. Verify network connectivity
4. Check authentication credentials

### Timeout Errors

**Error:**
```
Subrequest failed: operation timed out
```

**Cause:**
Request exceeded configured timeout.

**Solutions:**

1. Increase timeout:
```yaml
clients:
  slow_api:
    type: http
    timeout: 60  # Increase from 30
```

2. Optimize backend query
3. Use caching for slow operations
4. Check for backend performance issues

### Template Rendering Error

**Error:**
```
Failed to render template: invalid JSON
```

**Cause:**
Template produces invalid JSON.

**Solution:**
```yaml
# Bad - missing quotes around string value
template: '{"name": ${subrequest.user.body.name}}'

# Good - proper JSON quoting
template: '{"name": "${subrequest.user.body.name}"}'

# Bad - invalid escaping
template: '{"path": "C:\users\file"}'

# Good - escaped backslashes
template: '{"path": "C:\\users\\file"}'
```

---

## Performance Issues

### Slow Response Times

**Symptoms:**
- High latency
- Timeouts
- Slow overall performance

**Diagnosis:**
```bash
# Enable detailed logging
RUST_LOG=debug cargo run

# Check execution times
# Look for lines like:
# "Executing subrequest for client: api_client" (start)
# "HTTP response received: status=200" (end)
```

**Solutions:**

1. **Use Parallel Execution:**
```yaml
# Change from sequential to parallel
routes:
  - path: /dashboard
    execution_mode: parallel  # Faster
```

2. **Increase Connection Pools:**
```yaml
clients:
  api:
    max_connections: 50  # Increase from 10
```

3. **Add Caching:**
```yaml
subrequests:
  # Check cache first
  - name: cache_check
    client_id: redis
    type: redis
    operation:
      op: get
      key: "data:${request.path.id}"

  # Fetch only if not cached
  - name: source
    client_id: api
    type: http
    uri: /data/${request.path.id}
    depends_on: [cache_check]
```

4. **Optimize Database Queries:**
```yaml
# Bad - no indexes, slow query
query: "SELECT * FROM huge_table WHERE unindexed_column = $1"

# Good - indexed columns, limited results
query: "SELECT id, name FROM huge_table WHERE indexed_id = $1 LIMIT 100"
```

### Memory Usage

**Symptoms:**
- High memory consumption
- OOM errors

**Solutions:**

1. **Limit Response Sizes:**
```yaml
# Limit MongoDB results
operation:
  op: find
  filter: "{}"
  limit: 100  # Don't fetch everything

# Use pagination
query_params:
  page: "${request.query.page}"
  limit: "50"
```

2. **Filter Responses:**
```yaml
response_transform:
  # Only include needed fields
  include_fields:
    - id
    - name
    - email
```

3. **Reduce Connection Pools:**
```yaml
clients:
  db:
    max_connections: 10  # Reduce if memory constrained
```

### CPU Usage

**Symptoms:**
- High CPU usage
- Slow processing

**Solutions:**

1. **Reduce Logging:**
```bash
# Use warn or error in production
RUST_LOG=warn cargo run
```

2. **Optimize Regex:**
```yaml
# Complex regex in conditions can be slow
# Use simpler patterns when possible
condition:
  type: fieldmatches
  field: "id"
  pattern: "^[0-9]+$"  # Simple pattern
```

---

## Database Connectivity

### PostgreSQL Connection Failed

**Error:**
```
Failed to connect to postgres://...
```

**Checks:**
```bash
# Test connection
psql "postgres://user:password@localhost:5432/database"

# Check if PostgreSQL is running
systemctl status postgresql

# Check port
netstat -an | grep 5432
```

**Common Issues:**

1. **Authentication Failed:**
```yaml
# Check username and password
connection_string: "postgres://correct_user:correct_password@localhost:5432/db"
```

2. **Database Doesn't Exist:**
```bash
# Create database
createdb mydb
```

3. **SSL Issues:**
```yaml
# Try disabling SSL for local dev
connection_string: "postgres://user:pass@localhost/db?sslmode=disable"
```

### MongoDB Connection Failed

**Error:**
```
Failed to connect to mongodb://...
```

**Checks:**
```bash
# Test connection
mongo mongodb://localhost:27017

# Check if MongoDB is running
systemctl status mongod

# Check port
netstat -an | grep 27017
```

**Common Issues:**

1. **Authentication:**
```yaml
# Include credentials
connection_string: "mongodb://username:password@localhost:27017"
```

2. **Wrong Database:**
```yaml
# Verify database exists
clients:
  mongo:
    connection_string: "mongodb://localhost:27017"
    database: "existing_db"  # Must exist
```

### Redis Connection Failed

**Error:**
```
Failed to connect to redis://...
```

**Checks:**
```bash
# Test connection
redis-cli ping

# Check if Redis is running
systemctl status redis

# Test with password
redis-cli -a password ping
```

**Common Issues:**

1. **Password Required:**
```yaml
# Include password
connection_string: "redis://:password@localhost:6379"
```

2. **Wrong Database Number:**
```yaml
# Specify database
connection_string: "redis://localhost:6379/0"
```

---

## Debugging Tips

### Enable Debug Logging

```bash
# Full debug logging
RUST_LOG=debug cargo run

# Module-specific
RUST_LOG=pmp_gateway_api::routes=debug cargo run

# Trace level (very verbose)
RUST_LOG=trace cargo run

# Multiple modules
RUST_LOG=pmp_gateway_api=debug,sqlx=info cargo run
```

### Test Configuration

```bash
# Validate configuration
cargo run -- --validate-config

# Test with minimal config
cat > test-config.yaml <<EOF
clients:
  test:
    type: http
    base_url: "https://httpbin.org"

routes:
  - method: GET
    path: /test
    subrequests:
      - client_id: test
        type: http
        uri: /get
EOF

CONFIG_PATH=test-config.yaml cargo run
```

### Inspect Requests

```bash
# Test with curl and verbose output
curl -v http://localhost:3000/api/endpoint

# Test with headers
curl -v -H "Authorization: Bearer token" http://localhost:3000/api/endpoint

# Test POST with body
curl -v -X POST -H "Content-Type: application/json" -d '{"key":"value"}' http://localhost:3000/api/endpoint
```

### Check Logs

```bash
# Watch logs in real-time
RUST_LOG=debug cargo run 2>&1 | grep -i error

# Save logs to file
RUST_LOG=debug cargo run 2>&1 | tee app.log

# Filter specific errors
RUST_LOG=debug cargo run 2>&1 | grep -i "subrequest failed"
```

### Use Request Tracing

Enable request IDs in logs:
```bash
RUST_LOG=tower_http=debug,pmp_gateway_api=debug cargo run
```

Look for:
```
tower_http::trace: request started
pmp_gateway_api::routes: Handling request: GET
pmp_gateway_api::routes: Executing subrequest: user_data
tower_http::trace: response generated
```

### Benchmark Performance

```bash
# Install wrk
# Ubuntu
sudo apt-get install wrk

# macOS
brew install wrk

# Run benchmark
wrk -t4 -c100 -d30s http://localhost:3000/api/endpoint

# Results show:
# - Requests/sec
# - Latency distribution
# - Error rate
```

### Memory Profiling

```bash
# Install valgrind
sudo apt-get install valgrind

# Run with valgrind
valgrind --leak-check=full cargo run

# Or use heaptrack
heaptrack cargo run
```

---

## Common Patterns & Solutions

### Pattern: Data Not Available

**Problem:**
Sequential subrequest can't access previous result.

**Solution:**
```yaml
# Make sure previous subrequest has a name
subrequests:
  - name: user_data  # Add name!
    client_id: api
    type: http
    uri: /users/1

  - name: posts
    client_id: api
    type: http
    uri: /posts
    query_params:
      userId: "${subrequest.user_data.body.id}"  # Now works
```

### Pattern: Authentication Failures

**Problem:**
Backend requires authentication that's not being forwarded.

**Solution:**
```yaml
subrequests:
  - client_id: api
    type: http
    uri: /protected
    headers:
      # Forward auth header from incoming request
      Authorization: "${request.headers[\"authorization\"]}"
```

### Pattern: JSON Parsing Errors

**Problem:**
Response body is not valid JSON.

**Solution:**
```yaml
# Check the actual response
# Use debug logging to see raw responses
RUST_LOG=debug cargo run

# Consider response transformation
response_transform:
  # Only use specific fields that are valid JSON
  filter: "subrequests[0].body.data"
```

---

## Getting Help

### Gather Information

When reporting issues, include:

1. **Version:**
```bash
cargo run -- --version
```

2. **Configuration:**
```yaml
# Sanitized config (remove secrets)
```

3. **Logs:**
```bash
RUST_LOG=debug cargo run 2>&1 | tee error.log
```

4. **Error Message:**
```
Full error message and stack trace
```

5. **Steps to Reproduce:**
```
1. Start server with config X
2. Send request Y
3. Observe error Z
```

### Resources

- GitHub Issues: https://github.com/comfortablynumb/pmp-gateway-api/issues
- Documentation: See `docs/` directory
- Examples: See `config.*.yaml` files

### Self-Diagnosis Checklist

Before reporting:

- [ ] Checked logs with `RUST_LOG=debug`
- [ ] Validated YAML syntax
- [ ] Tested backend services independently
- [ ] Verified connection strings
- [ ] Checked firewall/network rules
- [ ] Tested with minimal configuration
- [ ] Reviewed this troubleshooting guide
- [ ] Checked for typos in client IDs and field names

---

## FAQ

### Q: Can I use environment variables in config?

A: Not directly in YAML, but you can use external tools:
```bash
# Use envsubst
envsubst < config.template.yaml > config.yaml
CONFIG_PATH=config.yaml cargo run
```

### Q: How do I handle large responses?

A: Use response transformation to filter:
```yaml
response_transform:
  include_fields: [id, name]  # Only keep needed fields
  exclude_fields: [large_blob]  # Remove large data
```

### Q: Can I add custom headers to all requests?

A: Yes, in client configuration:
```yaml
clients:
  api:
    headers:
      X-Custom-Header: "value"  # Added to all requests
```

### Q: How do I debug interpolation?

A: Enable debug logging and check the executed values:
```bash
RUST_LOG=debug cargo run
# Look for "Interpolated value: ..."
```

### Q: Can I use the same client for multiple routes?

A: Yes! Define once, use everywhere:
```yaml
clients:
  shared_api:
    type: http
    base_url: "..."

routes:
  - path: /route1
    subrequests:
      - client_id: shared_api  # Reuse
  - path: /route2
    subrequests:
      - client_id: shared_api  # Reuse
```

### Q: What happens if a subrequest fails?

A: The entire route fails and returns an error:
```json
{
  "error": "Subrequest failed: connection refused"
}
```

Consider using conditional execution for fallbacks.

### Q: How do I test without real backends?

A: Use public test APIs:
```yaml
clients:
  test:
    type: http
    base_url: "https://jsonplaceholder.typicode.com"
```

Or run local mock servers.
