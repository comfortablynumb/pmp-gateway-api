# PMP Gateway API - Complete Feature Documentation

This document provides comprehensive documentation for all features available in PMP Gateway API.

## Table of Contents

- [Client Types](#client-types)
- [Request Interpolation](#request-interpolation)
- [Subrequest Execution Modes](#subrequest-execution-modes)
- [Conditional Execution](#conditional-execution)
- [Response Transformation](#response-transformation)
- [Data Extraction](#data-extraction)

---

## Client Types

PMP Gateway API supports multiple backend client types, each optimized for specific use cases.

### HTTP Client

Connect to REST APIs and web services.

**Configuration:**
```yaml
clients:
  my_api:
    type: http
    base_url: "https://api.example.com"
    headers:
      User-Agent: "PMP-Gateway/1.0"
      Authorization: "Bearer token"
    min_connections: 1      # Minimum pool size (default: 1)
    max_connections: 10     # Maximum pool size (default: 10)
    timeout: 30             # Timeout in seconds (default: 30)
```

**Subrequest Configuration:**
```yaml
subrequests:
  - client_id: my_api
    type: http
    uri: /users/123                    # Path appended to base_url
    method: GET                        # HTTP method (default: GET)
    headers:                           # Additional headers
      X-Custom-Header: "value"
    body: '{"key": "value"}'          # Request body (optional)
    query_params:                      # Query parameters
      filter: "active"
      limit: "10"
```

**Features:**
- Connection pooling for performance
- Automatic header merging (client defaults + request specific)
- Full HTTP method support (GET, POST, PUT, DELETE, PATCH, etc.)
- Custom query parameters and headers per request
- Request body templating

---

### PostgreSQL Client

Connect to PostgreSQL databases with connection pooling and parameterized queries.

**Configuration:**
```yaml
clients:
  postgres_db:
    type: postgres
    connection_string: "postgres://user:password@localhost:5432/database"
    max_connections: 10     # Pool size (default: 10)
    timeout: 30             # Connection timeout (default: 30s)
```

**Subrequest Configuration:**
```yaml
subrequests:
  - client_id: postgres_db
    type: postgres
    query: "SELECT * FROM users WHERE id = $1 AND status = $2"
    params:
      - "${request.path.id}"
      - "active"
```

**Features:**
- Prepared statements with $1, $2, ... placeholders
- Automatic connection pooling
- Full CRUD support (SELECT, INSERT, UPDATE, DELETE)
- Transaction support via multiple queries
- Automatic type detection (strings, integers, booleans)
- Returns results as JSON arrays

**Result Format:**
```json
{
  "client_id": "postgres_db",
  "type": "sql",
  "rows": [
    {"id": 1, "name": "Alice", "email": "alice@example.com"},
    {"id": 2, "name": "Bob", "email": "bob@example.com"}
  ],
  "row_count": 2
}
```

---

### MySQL Client

Connect to MySQL/MariaDB databases.

**Configuration:**
```yaml
clients:
  mysql_db:
    type: mysql
    connection_string: "mysql://user:password@localhost:3306/database"
    max_connections: 10
    timeout: 30
```

**Subrequest Configuration:**
```yaml
subrequests:
  - client_id: mysql_db
    type: mysql
    query: "SELECT * FROM products WHERE category = ? AND price < ?"
    params:
      - "${request.query.category}"
      - "${request.query.max_price}"
```

**Features:**
- Same as PostgreSQL, but uses ? placeholders instead of $1, $2
- Compatible with MySQL and MariaDB
- Full transaction support

---

### SQLite Client

Connect to SQLite databases (ideal for local caching and embedded data).

**Configuration:**
```yaml
clients:
  local_cache:
    type: sqlite
    database_path: "sqlite://cache.db"
    max_connections: 5
```

**Subrequest Configuration:**
```yaml
subrequests:
  - client_id: local_cache
    type: sqlite
    query: "SELECT value FROM config WHERE key = ?"
    params:
      - "${request.path.key}"
```

**Features:**
- Perfect for embedded databases
- Low-latency local queries
- No network overhead
- Same query interface as other SQL databases

---

### MongoDB Client

Connect to MongoDB for document-oriented data storage.

**Configuration:**
```yaml
clients:
  mongo_db:
    type: mongodb
    connection_string: "mongodb://localhost:27017"
    database: "myapp"
    timeout: 30
```

**Supported Operations:**

#### Find
```yaml
subrequests:
  - client_id: mongo_db
    type: mongodb
    collection: "users"
    operation:
      op: find
      filter: '{"status": "active", "age": {"$gt": 18}}'
      limit: 100  # Optional
```

#### FindOne
```yaml
operation:
  op: findone
  filter: '{"email": "${request.body.email}"}'
```

#### Insert
```yaml
operation:
  op: insert
  document: '{"name": "${request.body.name}", "email": "${request.body.email}"}'
```

#### Update
```yaml
operation:
  op: update
  filter: '{"_id": "${request.path.id}"}'
  update: '{"$set": {"status": "updated"}}'
```

#### Delete
```yaml
operation:
  op: delete
  filter: '{"_id": "${request.path.id}"}'
```

**Features:**
- Full MongoDB query language support
- JSON filter and update expressions
- Supports all standard operators ($gt, $lt, $in, $set, etc.)
- Automatic document serialization

---

### Redis Client

Connect to Redis for caching and key-value storage.

**Configuration:**
```yaml
clients:
  redis_cache:
    type: redis
    connection_string: "redis://localhost:6379"
    timeout: 10
```

**Supported Operations:**

#### GET
```yaml
operation:
  op: get
  key: "user:${request.path.id}"
```

#### SET
```yaml
operation:
  op: set
  key: "session:${request.headers[\"session-id\"]}"
  value: "${request.body}"
  expiration: 3600  # TTL in seconds (optional)
```

#### DEL
```yaml
operation:
  op: del
  key: "cache:${request.path.key}"
```

#### EXISTS
```yaml
operation:
  op: exists
  key: "user:${request.path.id}"
```

#### HGET (Hash Get)
```yaml
operation:
  op: hget
  key: "user:${request.path.id}"
  field: "email"
```

#### HSET (Hash Set)
```yaml
operation:
  op: hset
  key: "user:${request.path.id}"
  field: "last_login"
  value: "2024-01-01"
```

**Features:**
- Full hash operations support
- Automatic expiration (TTL) support
- Connection pooling for performance
- Ideal for session management and caching

---

## Request Interpolation

Dynamic value insertion from incoming requests using `${...}` syntax.

### Available Variables

#### Request Headers
```yaml
# Access any request header
headers:
  Authorization: "${request.headers[\"authorization\"]}"
  X-User-ID: "${request.headers[\"x-user-id\"]}"
```

#### Path Parameters
```yaml
# For route: /users/:id/posts/:post_id
uri: /api/users/${request.path.id}/posts/${request.path.post_id}
```

#### Query Parameters
```yaml
# For request: /search?q=rust&limit=10
query_params:
  search: "${request.query.q}"
  max: "${request.query.limit}"
```

#### Request Body
```yaml
# Access the entire request body
body: "${request.body}"

# Or embed it in a template
body: '{"data": ${request.body}, "timestamp": "now"}'
```

#### HTTP Method
```yaml
# Access the HTTP method
headers:
  X-Original-Method: "${request.method}"
```

#### Subrequest Results (Sequential Execution)
```yaml
# Access data from previous named subrequests
body: '{"user_id": "${subrequest.user_data.body.id}"}'
uri: /posts/${subrequest.first_post.body.0.id}
```

### Interpolation Examples

**Complex Template:**
```yaml
body: |
  {
    "user": {
      "id": "${request.path.id}",
      "email": "${subrequest.user_data.body.email}",
      "posts": ${subrequest.user_posts.body}
    },
    "metadata": {
      "method": "${request.method}",
      "timestamp": "now"
    }
  }
```

---

## Subrequest Execution Modes

Control how multiple subrequests are executed: sequentially or in parallel.

### Parallel Mode (Default)

Execute all subrequests simultaneously for maximum performance.

```yaml
routes:
  - method: GET
    path: /dashboard/:user_id
    execution_mode: parallel
    subrequests:
      - name: profile
        # Fetches profile
      - name: posts
        # Fetches posts
      - name: stats
        # Fetches stats
# All three execute at the same time
```

**Benefits:**
- Maximum performance
- Minimal latency
- Ideal for independent requests

**Limitations:**
- Cannot access data from other subrequests
- All execute with the same initial context

### Sequential Mode

Execute subrequests one after another, allowing data dependencies.

```yaml
routes:
  - method: GET
    path: /user-workflow/:id
    execution_mode: sequential
    subrequests:
      - name: user
        # Fetches user data
      - name: posts
        # Uses user.id to fetch posts
        query_params:
          userId: "${subrequest.user.body.id}"
      - name: cache
        # Caches the results
        operation:
          key: "user:${subrequest.user.body.id}"
```

**Benefits:**
- Full data access from previous requests
- Complex multi-step workflows
- Dependent operations

**Limitations:**
- Higher latency (serial execution)
- Total time = sum of all subrequest times

### Hybrid Mode: Dependencies

Even in parallel mode, use `depends_on` to create execution waves.

```yaml
routes:
  - method: GET
    path: /smart-fetch/:id
    execution_mode: parallel
    subrequests:
      # Wave 1: Executes first
      - name: user
        client_id: api
        type: http
        uri: /users/${request.path.id}

      # Wave 2: Both wait for 'user', but execute in parallel
      - name: posts
        depends_on: [user]
        query_params:
          userId: "${subrequest.user.body.id}"

      - name: comments
        depends_on: [user]
        query_params:
          userId: "${subrequest.user.body.id}"
```

**Benefits:**
- Combines performance and dependencies
- Optimal execution time
- Flexible workflow design

**Dependency Resolution:**
- Automatic topological sorting
- Circular dependency detection
- Wave-based parallel execution

---

## Conditional Execution

Execute subrequests conditionally based on runtime criteria.

### Condition Types

#### Always
```yaml
condition:
  type: always
```

#### Field Exists
```yaml
condition:
  type: fieldexists
  field: "user_id"  # Checks path_params and query_params
```

#### Field Equals
```yaml
condition:
  type: fieldequals
  field: "status"
  value: "active"
```

#### Field Matches (Regex)
```yaml
condition:
  type: fieldmatches
  field: "email"
  pattern: "^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}$"
```

#### Header Exists
```yaml
condition:
  type: headerexists
  header: "Authorization"
```

#### Header Equals
```yaml
condition:
  type: headerequals
  header: "X-Role"
  value: "admin"
```

#### Query Exists
```yaml
condition:
  type: queryexists
  param: "debug"
```

#### Query Equals
```yaml
condition:
  type: queryequals
  param: "format"
  value: "json"
```

### Logical Operators

#### AND
```yaml
condition:
  type: and
  conditions:
    - type: headerexists
      header: "Authorization"
    - type: headerequals
      header: "X-Role"
      value: "admin"
```

#### OR
```yaml
condition:
  type: or
  conditions:
    - type: headerequals
      header: "X-Subscription"
      value: "premium"
    - type: headerequals
      header: "X-Role"
      value: "admin"
```

#### NOT
```yaml
condition:
  type: not
  condition:
    type: headerexists
    header: "X-Internal-Request"
```

### Complex Example
```yaml
subrequests:
  - name: premium_content
    client_id: content_api
    type: http
    uri: /premium/articles
    condition:
      type: and
      conditions:
        # Must be authenticated
        - type: headerexists
          header: "Authorization"
        # Must be premium OR admin
        - type: or
          conditions:
            - type: headerequals
              header: "X-Subscription"
              value: "premium"
            - type: headerequals
              header: "X-Role"
              value: "admin"
```

---

## Response Transformation

Transform and filter the aggregated response from all subrequests.

### Filter (JSONPath-like)

Extract specific data from the response.

```yaml
response_transform:
  filter: "subrequests[0].body"  # Get body from first subrequest
```

**Complex Filtering:**
```yaml
response_transform:
  filter: "subrequests.user_data.body.posts"
```

### Field Mappings

Rename fields in the response.

```yaml
response_transform:
  field_mappings:
    subrequests: "data_sources"
    count: "total"
    client_id: "source"
```

**Before:**
```json
{
  "subrequests": [...],
  "count": 3,
  "client_id": "api"
}
```

**After:**
```json
{
  "data_sources": [...],
  "total": 3,
  "source": "api"
}
```

### Include Fields

Only include specific fields (whitelist).

```yaml
response_transform:
  include_fields:
    - "subrequests"
    - "count"
```

### Exclude Fields

Remove specific fields (blacklist).

```yaml
response_transform:
  exclude_fields:
    - "internal_id"
    - "password_hash"
    - "secret_key"
```

### Custom Templates

Create custom response format with interpolation.

```yaml
response_transform:
  template: |
    {
      "user": {
        "name": "${subrequest.user.body.name}",
        "email": "${subrequest.user.body.email}"
      },
      "stats": {
        "posts": ${subrequest.posts.count},
        "comments": ${subrequest.comments.count}
      }
    }
```

### Combined Transformations

```yaml
response_transform:
  filter: "subrequests"
  field_mappings:
    body: "data"
    status: "http_status"
  exclude_fields:
    - "headers"
    - "internal_metadata"
  template: |
    {
      "results": ${response},
      "processed_at": "now"
    }
```

---

## Data Extraction

Extract and reference data from previous subrequest results.

### Naming Subrequests

```yaml
subrequests:
  - name: user_data  # Name this subrequest
    client_id: api
    type: http
    uri: /users/1
```

### Accessing Subrequest Results

**Basic Access:**
```yaml
# Access the entire result
value: "${subrequest.user_data}"

# Access nested fields
email: "${subrequest.user_data.body.email}"
user_id: "${subrequest.user_data.body.id}"
```

**HTTP Response Structure:**
```json
{
  "client_id": "api",
  "type": "http",
  "status": 200,
  "body": {...},  # Response body
  "headers": {...}
}
```

**SQL Response Structure:**
```json
{
  "client_id": "db",
  "type": "sql",
  "rows": [...],  # Array of rows
  "row_count": 10
}
```

### Nested Navigation

```yaml
# Multi-level nesting
company_name: "${subrequest.user.body.company.name}"
city: "${subrequest.user.body.address.city}"

# Access SQL results
user_id: "${subrequest.new_user.rows.0.id}"
user_name: "${subrequest.new_user.rows.0.name}"
```

### Array Indexing

```yaml
# Access array elements by index
first_post: "${subrequest.posts.body.0.title}"
second_post: "${subrequest.posts.body.1.title}"

# Access nested arrays
first_comment: "${subrequest.posts.body.0.comments.0.text}"
```

### Complete Example

```yaml
routes:
  - method: POST
    path: /create-and-notify
    execution_mode: sequential
    subrequests:
      # Step 1: Create user
      - name: create_user
        client_id: db
        type: postgres
        query: "INSERT INTO users (name, email) VALUES ($1, $2) RETURNING id, name, email"
        params:
          - "${request.body.name}"
          - "${request.body.email}"

      # Step 2: Cache user
      - name: cache_user
        client_id: redis
        type: redis
        operation:
          op: set
          key: "user:${subrequest.create_user.rows.0.id}"
          value: '{"id": ${subrequest.create_user.rows.0.id}, "name": "${subrequest.create_user.rows.0.name}"}'
          expiration: 3600

      # Step 3: Send notification
      - name: send_notification
        client_id: api
        type: http
        uri: /notifications
        method: POST
        body: |
          {
            "user_id": ${subrequest.create_user.rows.0.id},
            "email": "${subrequest.create_user.rows.0.email}",
            "message": "Welcome ${subrequest.create_user.rows.0.name}!"
          }
```

---

## Best Practices

### Performance

1. **Use Parallel Mode** when requests are independent
2. **Use Connection Pooling** - configure appropriate pool sizes
3. **Cache Frequently Accessed Data** using Redis
4. **Use Dependencies Wisely** - only when necessary

### Security

1. **Validate Input** - use conditions to validate requests
2. **Sanitize Interpolations** - be careful with user input in queries
3. **Use Parameterized Queries** - always for SQL to prevent injection
4. **Hide Sensitive Data** - use exclude_fields in transformations

### Maintainability

1. **Name Your Subrequests** - makes debugging easier
2. **Use Meaningful Client IDs** - descriptive names
3. **Document Complex Workflows** - add comments in YAML
4. **Keep Routes Focused** - avoid too many subrequests per route

### Error Handling

1. **Set Appropriate Timeouts** - prevent hanging requests
2. **Handle Failures** - use conditional execution for fallbacks
3. **Monitor Logs** - use RUST_LOG for debugging
4. **Test Dependencies** - ensure no circular references

---

## Migration Guide

### From Simple Proxy

**Before (Manual):**
```yaml
routes:
  - method: GET
    path: /users/:id
    subrequests:
      - client_id: api
        type: http
        uri: /users/${request.path.id}
```

**After (With Caching):**
```yaml
routes:
  - method: GET
    path: /users/:id
    execution_mode: sequential
    subrequests:
      - name: check_cache
        client_id: redis
        type: redis
        operation:
          op: get
          key: "user:${request.path.id}"

      - name: fetch_api
        client_id: api
        type: http
        uri: /users/${request.path.id}

      - name: update_cache
        client_id: redis
        type: redis
        operation:
          op: set
          key: "user:${request.path.id}"
          value: "${subrequest.fetch_api.body}"
          expiration: 3600
        depends_on: [fetch_api]
```

---

For more examples, see the configuration files in the repository:
- `config.example.yaml` - All client types and basic features
- `config.sequential.example.yaml` - Sequential/parallel execution examples
