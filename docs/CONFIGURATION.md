# Configuration Guide

Complete guide to configuring PMP Gateway API.

## Table of Contents

- [File Structure](#file-structure)
- [Environment Variables](#environment-variables)
- [Client Configuration](#client-configuration)
- [Route Configuration](#route-configuration)
- [Advanced Patterns](#advanced-patterns)

---

## File Structure

PMP Gateway API uses YAML configuration files.

### Basic Structure

```yaml
# config.yaml
clients:
  # Define all backend clients here
  client_id_1:
    type: http
    # client-specific configuration

  client_id_2:
    type: postgres
    # client-specific configuration

routes:
  # Define all routes here
  - method: GET
    path: /api/endpoint
    execution_mode: parallel
    subrequests:
      - client_id: client_id_1
        # subrequest configuration
```

### Configuration File Location

By default, the application looks for `config.yaml` in the current directory.

**Custom Location:**
```bash
CONFIG_PATH=/path/to/config.yaml ./pmp-gateway-api
```

---

## Environment Variables

### Core Settings

| Variable | Description | Default |
|----------|-------------|---------|
| `CONFIG_PATH` | Path to YAML configuration file | `config.yaml` |
| `HOST` | Server bind address | `0.0.0.0` |
| `PORT` | Server port | `3000` |
| `RUST_LOG` | Logging level | `info` |

### Usage Examples

**Development:**
```bash
export RUST_LOG=debug
export HOST=127.0.0.1
export PORT=8080
./pmp-gateway-api
```

**Production:**
```bash
export RUST_LOG=warn
export CONFIG_PATH=/etc/pmp-gateway/config.yaml
./pmp-gateway-api
```

**Docker:**
```bash
docker run -e RUST_LOG=info -e PORT=3000 -v /path/to/config.yaml:/app/config.yaml pmp-gateway-api
```

### Logging Levels

```bash
# Minimal logging
RUST_LOG=error

# Production
RUST_LOG=warn

# Default
RUST_LOG=info

# Verbose
RUST_LOG=debug

# Very verbose (includes dependencies)
RUST_LOG=trace

# Module-specific
RUST_LOG=pmp_gateway_api=debug,tower_http=info

# Multiple modules
RUST_LOG=pmp_gateway_api::routes=debug,pmp_gateway_api::clients=info
```

---

## Client Configuration

### HTTP Client

```yaml
clients:
  api_client:
    type: http
    base_url: "https://api.example.com"

    # Optional: Default headers for all requests
    headers:
      User-Agent: "PMP-Gateway/1.0"
      Accept: "application/json"
      Authorization: "Bearer static-token"

    # Optional: Connection pool settings
    min_connections: 1      # Default: 1
    max_connections: 10     # Default: 10

    # Optional: Timeout in seconds
    timeout: 30             # Default: 30
```

**Best Practices:**
- Use appropriate pool sizes based on expected load
- Set conservative timeouts to prevent hanging
- Use environment variables for sensitive tokens (not shown in example)

### PostgreSQL Client

```yaml
clients:
  postgres_db:
    type: postgres
    connection_string: "postgres://user:password@localhost:5432/database"

    # Optional
    max_connections: 10     # Default: 10
    timeout: 30             # Default: 30
```

**Connection String Format:**
```
postgres://username:password@hostname:port/database?sslmode=require
```

**SSL Options:**
- `sslmode=disable` - No SSL
- `sslmode=require` - Require SSL
- `sslmode=prefer` - Prefer SSL

### MySQL Client

```yaml
clients:
  mysql_db:
    type: mysql
    connection_string: "mysql://user:password@localhost:3306/database"
    max_connections: 10
    timeout: 30
```

**Connection String Format:**
```
mysql://username:password@hostname:port/database?ssl-mode=required
```

### SQLite Client

```yaml
clients:
  local_db:
    type: sqlite
    database_path: "sqlite://path/to/database.db"
    max_connections: 5      # SQLite benefits from smaller pools
```

**Path Options:**
```yaml
# Relative path
database_path: "sqlite://data/cache.db"

# Absolute path
database_path: "sqlite:///var/lib/app/database.db"

# In-memory (testing only)
database_path: "sqlite::memory:"
```

### MongoDB Client

```yaml
clients:
  mongo_db:
    type: mongodb
    connection_string: "mongodb://localhost:27017"
    database: "myapp"
    timeout: 30
```

**Connection String Options:**
```yaml
# Simple
connection_string: "mongodb://localhost:27017"

# With authentication
connection_string: "mongodb://username:password@localhost:27017"

# Replica set
connection_string: "mongodb://host1:27017,host2:27017,host3:27017/?replicaSet=mySet"

# With options
connection_string: "mongodb://localhost:27017/?maxPoolSize=20&w=majority"
```

### Redis Client

```yaml
clients:
  redis_cache:
    type: redis
    connection_string: "redis://localhost:6379"
    timeout: 10
```

**Connection String Options:**
```yaml
# Simple
connection_string: "redis://localhost:6379"

# With password
connection_string: "redis://:password@localhost:6379"

# Specific database
connection_string: "redis://localhost:6379/0"

# TLS
connection_string: "rediss://localhost:6380"
```

---

## Route Configuration

### Basic Route

```yaml
routes:
  - method: GET
    path: /api/users/:id
    subrequests:
      - client_id: api_client
        type: http
        uri: /users/${request.path.id}
        method: GET
```

### Path Parameters

```yaml
routes:
  # Single parameter
  - method: GET
    path: /users/:id
    # Access with: ${request.path.id}

  # Multiple parameters
  - method: GET
    path: /users/:user_id/posts/:post_id
    # Access with: ${request.path.user_id} and ${request.path.post_id}

  # Wildcard (catches all)
  - method: GET
    path: /files/*path
    # Access with: ${request.path.path}
```

### Execution Modes

```yaml
routes:
  # Parallel (default) - all subrequests execute simultaneously
  - method: GET
    path: /parallel
    execution_mode: parallel
    subrequests: [...]

  # Sequential - execute one by one
  - method: GET
    path: /sequential
    execution_mode: sequential
    subrequests: [...]
```

### Subrequest Configuration

#### Named Subrequests

```yaml
subrequests:
  - name: user_data          # Required for data extraction
    client_id: api_client
    type: http
    uri: /users/1
```

#### Dependencies

```yaml
subrequests:
  - name: user
    client_id: api
    # No dependencies - executes immediately

  - name: posts
    client_id: api
    depends_on: [user]       # Waits for 'user' to complete

  - name: enriched_posts
    client_id: db
    depends_on: [user, posts]  # Waits for both
```

#### Conditional Execution

```yaml
subrequests:
  - name: admin_data
    client_id: api
    type: http
    uri: /admin/stats
    condition:
      type: headerequals
      header: "X-Role"
      value: "admin"
```

### Response Transformation

```yaml
routes:
  - method: GET
    path: /transformed
    subrequests: [...]
    response_transform:
      # Extract specific data
      filter: "subrequests[0].body"

      # Rename fields
      field_mappings:
        old_name: new_name

      # Include only these fields
      include_fields:
        - field1
        - field2

      # Exclude these fields
      exclude_fields:
        - sensitive_data

      # Custom template
      template: |
        {
          "result": ${response},
          "timestamp": "now"
        }
```

---

## Advanced Patterns

### Multi-Stage Workflows

```yaml
routes:
  - method: POST
    path: /create-user-workflow
    execution_mode: sequential
    subrequests:
      # Stage 1: Create user
      - name: create
        client_id: db
        type: postgres
        query: "INSERT INTO users (...) RETURNING id"
        params: [...]

      # Stage 2: Cache user
      - name: cache
        client_id: redis
        type: redis
        operation:
          op: set
          key: "user:${subrequest.create.rows.0.id}"
          value: "${subrequest.create.rows.0}"
          expiration: 3600

      # Stage 3: Send notification
      - name: notify
        client_id: notification_api
        type: http
        uri: /notify
        body: '{"user_id": ${subrequest.create.rows.0.id}}'
```

### Cache-First Pattern

```yaml
routes:
  - method: GET
    path: /cached/:id
    execution_mode: sequential
    subrequests:
      # Try cache first
      - name: cache_check
        client_id: redis
        type: redis
        operation:
          op: get
          key: "item:${request.path.id}"

      # Fetch from source if needed
      - name: source_fetch
        client_id: api
        type: http
        uri: /items/${request.path.id}

      # Update cache
      - name: cache_update
        client_id: redis
        type: redis
        operation:
          op: set
          key: "item:${request.path.id}"
          value: "${subrequest.source_fetch.body}"
          expiration: 3600
        depends_on: [source_fetch]
```

### Fan-Out Pattern

```yaml
routes:
  - method: GET
    path: /dashboard/:user_id
    execution_mode: parallel
    subrequests:
      # All execute simultaneously
      - name: profile
        client_id: user_api
        type: http
        uri: /users/${request.path.user_id}

      - name: posts
        client_id: content_api
        type: http
        uri: /posts?user=${request.path.user_id}

      - name: stats
        client_id: analytics_db
        type: postgres
        query: "SELECT * FROM user_stats WHERE user_id = $1"
        params: ["${request.path.user_id}"]

      - name: notifications
        client_id: notification_api
        type: http
        uri: /notifications/${request.path.user_id}
```

### Aggregation with Dependencies

```yaml
routes:
  - method: GET
    path: /enriched/:id
    execution_mode: parallel
    subrequests:
      # Wave 1: Independent requests
      - name: user
        client_id: api
        type: http
        uri: /users/${request.path.id}

      - name: settings
        client_id: db
        type: postgres
        query: "SELECT * FROM settings WHERE user_id = $1"
        params: ["${request.path.id}"]

      # Wave 2: Dependent on wave 1
      - name: user_posts
        client_id: api
        type: http
        uri: /posts?userId=${subrequest.user.body.id}
        depends_on: [user]

      - name: user_friends
        client_id: api
        type: http
        uri: /friends?userId=${subrequest.user.body.id}
        depends_on: [user]
```

### Conditional Branching

```yaml
routes:
  - method: GET
    path: /content/:id
    execution_mode: parallel
    subrequests:
      # Always execute
      - name: basic_content
        client_id: content_api
        type: http
        uri: /content/${request.path.id}

      # Only for premium users
      - name: premium_content
        client_id: premium_api
        type: http
        uri: /premium/${request.path.id}
        condition:
          type: or
          conditions:
            - type: headerequals
              header: "X-Subscription"
              value: "premium"
            - type: headerequals
              header: "X-Role"
              value: "admin"

      # Only for admins
      - name: analytics
        client_id: analytics_db
        type: postgres
        query: "SELECT * FROM content_analytics WHERE id = $1"
        params: ["${request.path.id}"]
        condition:
          type: headerequals
          header: "X-Role"
          value: "admin"
```

---

## Validation

The gateway validates configuration on startup:

### Client Validation
- All client IDs are unique
- Required fields present
- Connection strings properly formatted

### Route Validation
- All referenced `client_id` exist
- No circular dependencies in `depends_on`
- Valid execution modes
- Proper condition syntax

### Error Examples

```
Error: Route /users/:id references unknown client_id: missing_client
Error: Circular dependency detected in subrequests
Error: Invalid execution mode: invalid_mode
```

---

## Performance Tuning

### Connection Pools

```yaml
# High traffic
clients:
  api:
    type: http
    max_connections: 50

# Database
clients:
  db:
    type: postgres
    max_connections: 20    # Based on database limits

# Redis
clients:
  cache:
    type: redis
    # Redis is single-threaded, smaller pool is fine
```

### Timeouts

```yaml
# Fast API
clients:
  fast_api:
    type: http
    timeout: 5

# Slow queries
clients:
  analytics:
    type: postgres
    timeout: 60

# Default
clients:
  standard:
    type: http
    timeout: 30
```

### Execution Modes

```yaml
# Use parallel for independent requests
routes:
  - path: /dashboard
    execution_mode: parallel
    # Faster total execution

# Use sequential only when needed
routes:
  - path: /workflow
    execution_mode: sequential
    # When data dependencies exist
```

---

## Security Best Practices

### 1. Never Hardcode Secrets

**Bad:**
```yaml
clients:
  db:
    connection_string: "postgres://admin:password123@localhost/db"
```

**Good:**
```bash
# Use environment variables
export DB_CONNECTION="postgres://..."
# Then reference in code or use templating
```

### 2. Use Parameterized Queries

**Bad:**
```yaml
query: "SELECT * FROM users WHERE id = '${request.path.id}'"
```

**Good:**
```yaml
query: "SELECT * FROM users WHERE id = $1"
params:
  - "${request.path.id}"
```

### 3. Validate Input

```yaml
subrequests:
  - name: validated
    # Only execute if ID is numeric
    condition:
      type: fieldmatches
      field: "id"
      pattern: "^[0-9]+$"
```

### 4. Filter Sensitive Data

```yaml
response_transform:
  exclude_fields:
    - password_hash
    - secret_key
    - internal_id
```

---

## Testing Configuration

### Validation

```bash
# Check syntax
cargo run -- --validate-config

# Dry run
RUST_LOG=debug cargo run
```

### Minimal Test Config

```yaml
clients:
  test_api:
    type: http
    base_url: "https://jsonplaceholder.typicode.com"

routes:
  - method: GET
    path: /test/:id
    subrequests:
      - client_id: test_api
        type: http
        uri: /users/${request.path.id}
```

### Testing with cURL

```bash
# Start server
cargo run

# Test endpoint
curl http://localhost:3000/test/1

# With headers
curl -H "Authorization: Bearer token" http://localhost:3000/test/1

# With query params
curl "http://localhost:3000/test/1?filter=active&limit=10"
```

---

For complete examples, see:
- `config.yaml` - Simple test configuration
- `config.example.yaml` - Comprehensive examples
- `config.sequential.example.yaml` - Execution mode examples
