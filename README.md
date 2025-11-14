# PMP Gateway API

[![CI](https://github.com/comfortablynumb/pmp-gateway-api/actions/workflows/ci.yml/badge.svg)](https://github.com/comfortablynumb/pmp-gateway-api/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

A powerful, YAML-configurable API gateway built in Rust that allows you to configure endpoints to proxy, aggregate, transform, and orchestrate requests across multiple backend services and databases. Part of the Poor Man's Platform ecosystem.

## Features

- **YAML-based Configuration**: Define routes, clients, and request mappings entirely in YAML
- **Multiple Client Types**: HTTP, PostgreSQL, MySQL, SQLite, MongoDB, Redis
- **Connection Pooling**: Efficient connection management with configurable pool sizes for all client types
- **Request Interpolation**: Dynamic request mapping using template syntax like `${request.headers["Authorization"]}`
- **Sequential & Parallel Execution**: Configure subrequests to execute sequentially or in parallel with dependency management
- **Data Extraction**: Extract and reference data from previous subrequests using JSONPath-like syntax
- **Conditional Execution**: Execute subrequests based on conditions (field checks, header validation, regex matching)
- **Response Transformation**: Filter, map, and transform responses with templates
- **Subrequests**: Execute multiple backend requests for a single incoming request
- **Header Forwarding**: Pass through or transform headers from incoming requests
- **Query Parameter Mapping**: Map and transform query parameters
- **Body Templating**: Template request bodies with data from incoming and previous requests

## Quick Start

### Installation

```bash
# Clone the repository
git clone https://github.com/comfortablynumb/pmp-gateway-api
cd pmp-gateway-api

# Build the project
cargo build --release

# Run the server
cargo run --release
```

### Configuration

Create a `config.yaml` file:

```yaml
clients:
  # HTTP Client
  my_api:
    type: http
    base_url: "https://api.example.com"
    headers:
      User-Agent: "PMP-Gateway/1.0"
    max_connections: 20
    timeout: 30

  # PostgreSQL Client
  postgres:
    type: postgres
    connection_string: "postgres://user:pass@localhost/dbname"
    max_connections: 10

  # MongoDB Client
  mongodb:
    type: mongodb
    connection_string: "mongodb://localhost:27017"
    database: "mydb"
    max_connections: 10

  # Redis Client
  redis:
    type: redis
    connection_string: "redis://localhost:6379"

routes:
  - method: GET
    path: /users/:id
    execution_mode: sequential  # or 'parallel'
    subrequests:
      - name: user_data
        client_id: my_api
        type: http
        uri: /users/${request.path.id}
        method: GET
        headers:
          Authorization: "${request.headers[\"authorization\"]}"

      - name: user_posts
        client_id: postgres
        type: sql
        query: "SELECT * FROM posts WHERE user_id = $1"
        params:
          - "${subrequest.user_data.body.id}"
        depends_on: [user_data]
```

### Running

```bash
# Use default config.yaml
cargo run

# Or specify a custom config file
CONFIG_PATH=my-config.yaml cargo run

# Customize host and port
HOST=127.0.0.1 PORT=8080 cargo run
```

## Core Concepts

### Client Types

PMP Gateway supports six client types, each with connection pooling and configurable timeouts:

| Client Type | Description | Use Cases |
|------------|-------------|-----------|
| **HTTP** | REST API client | API proxying, service aggregation |
| **PostgreSQL** | SQL database | Query relational data |
| **MySQL** | SQL database | Query relational data |
| **SQLite** | SQL database | Lightweight local storage |
| **MongoDB** | NoSQL document database | Query document stores |
| **Redis** | Key-value store | Caching, session storage |

### Execution Modes

- **Sequential**: Subrequests execute one after another, allowing later requests to use data from earlier ones
- **Parallel**: Subrequests execute concurrently with automatic dependency resolution

### Interpolation Syntax

Reference data from incoming requests and previous subrequests:

```yaml
# Request data
${request.headers["Authorization"]}
${request.path.user_id}
${request.query.filter}
${request.body}

# Previous subrequest data
${subrequest.user_data.body.id}
${subrequest.cache_check.value}
```

## Documentation

- **[Features Guide](docs/FEATURES.md)** - Comprehensive guide to all features
- **[Configuration Reference](docs/CONFIGURATION.md)** - Complete configuration documentation
- **[Troubleshooting](docs/TROUBLESHOOTING.md)** - Common issues and solutions

## Example Configurations

- `config.yaml` - Simple HTTP proxy example
- `config.example.yaml` - Comprehensive examples with all client types
- `config.sequential.example.yaml` - Sequential and parallel execution patterns

## Use Cases

### 1. API Gateway with Caching

Cache API responses in Redis:

```yaml
routes:
  - method: GET
    path: /api/data/:id
    execution_mode: sequential
    subrequests:
      # Check cache first
      - name: cache_check
        client_id: redis
        type: redis
        operation:
          op: get
          key: "data:${request.path.id}"

      # Fetch from API if not cached
      - name: api_fetch
        client_id: api
        type: http
        uri: /data/${request.path.id}
        condition:
          type: fieldexists
          field: "subrequest.cache_check.value"
          negate: true

      # Cache the result
      - name: cache_set
        client_id: redis
        type: redis
        operation:
          op: set
          key: "data:${request.path.id}"
          value: "${subrequest.api_fetch.body}"
          expiration: 3600
        depends_on: [api_fetch]
```

### 2. Multi-Database Aggregation

Combine data from SQL and NoSQL databases:

```yaml
routes:
  - method: GET
    path: /users/:id/complete
    execution_mode: parallel
    subrequests:
      - name: user_profile
        client_id: postgres
        type: sql
        query: "SELECT * FROM users WHERE id = $1"
        params: ["${request.path.id}"]

      - name: user_activity
        client_id: mongodb
        type: mongo
        collection: "activities"
        operation:
          op: find
          filter: '{"user_id": "${request.path.id}"}'
          limit: 10
```

### 3. Sequential Data Pipeline

Execute operations in sequence with data dependencies:

```yaml
routes:
  - method: POST
    path: /orders
    execution_mode: sequential
    subrequests:
      # Validate user
      - name: user
        client_id: api
        type: http
        uri: /users/${request.body.user_id}

      # Check inventory
      - name: inventory
        client_id: postgres
        type: sql
        query: "SELECT * FROM products WHERE id = $1"
        params: ["${request.body.product_id}"]

      # Create order
      - name: order
        client_id: postgres
        type: sql
        query: "INSERT INTO orders (user_id, product_id, quantity) VALUES ($1, $2, $3)"
        params:
          - "${subrequest.user.body.id}"
          - "${request.body.product_id}"
          - "${request.body.quantity}"
        depends_on: [user, inventory]
```

### 4. Conditional Execution

Execute different logic based on conditions:

```yaml
routes:
  - method: GET
    path: /content/:id
    subrequests:
      - name: premium_content
        client_id: api
        type: http
        uri: /premium/${request.path.id}
        condition:
          type: headerexists
          header: "X-Premium-User"

      - name: free_content
        client_id: api
        type: http
        uri: /free/${request.path.id}
        condition:
          type: headerexists
          header: "X-Premium-User"
          negate: true
```

## Environment Variables

- `CONFIG_PATH`: Path to configuration file (default: `config.yaml`)
- `HOST`: Server host (default: `0.0.0.0`)
- `PORT`: Server port (default: `3000`)
- `RUST_LOG`: Logging level (e.g., `debug`, `info`, `warn`, `error`)

## Built With

- **[Rust](https://www.rust-lang.org/)** - Systems programming language
- **[Axum](https://github.com/tokio-rs/axum)** - Web application framework
- **[Tokio](https://tokio.rs/)** - Async runtime
- **[SQLx](https://github.com/launchbadge/sqlx)** - SQL database driver
- **[MongoDB Driver](https://github.com/mongodb/mongo-rust-driver)** - MongoDB client
- **[Redis](https://github.com/redis-rs/redis-rs)** - Redis client
- **[Reqwest](https://github.com/seanmonstar/reqwest)** - HTTP client
- **[Serde](https://serde.rs/)** - Serialization framework

## Development

### Running Tests

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_name
```

### Running with Debug Logging

```bash
# Full debug logging
RUST_LOG=debug cargo run

# Module-specific logging
RUST_LOG=pmp_gateway_api=debug,sqlx=info cargo run

# Trace level (very verbose)
RUST_LOG=trace cargo run
```

### Linting and Formatting

```bash
# Check formatting
cargo fmt --all -- --check

# Format code
cargo fmt --all

# Run clippy
cargo clippy --all-targets --all-features -- -D warnings
```

### Building for Production

```bash
# Build release binary
cargo build --release

# Binary will be at target/release/pmp-gateway-api
./target/release/pmp-gateway-api
```

## CI/CD

The project uses GitHub Actions for continuous integration:

- **Format Check**: Ensures code is properly formatted with `rustfmt`
- **Lint**: Runs `clippy` with strict warnings
- **Test**: Runs tests on stable, beta, and nightly Rust
- **Build**: Builds on Ubuntu, macOS, and Windows
- **Coverage**: Generates code coverage reports with `tarpaulin`
- **Security Audit**: Checks for vulnerabilities with `cargo-audit`
- **Documentation**: Validates documentation builds

## Roadmap

### Completed Features
- [x] HTTP client with connection pooling
- [x] SQL database clients (PostgreSQL, MySQL, SQLite)
- [x] NoSQL database clients (MongoDB)
- [x] Redis/cache client
- [x] Response transformation and filtering
- [x] Conditional subrequest execution
- [x] Sequential and parallel execution modes
- [x] Request interpolation and data extraction

### Future Enhancements
- [ ] Rate limiting and throttling
- [ ] Circuit breaker pattern
- [ ] Metrics and monitoring (Prometheus/OpenTelemetry)
- [ ] GraphQL support
- [ ] WebSocket proxying
- [ ] Request/response logging and audit trails
- [ ] Dynamic configuration reloading
- [ ] A/B testing and canary deployments
- [ ] Authentication and authorization middleware
- [ ] Request validation and schema enforcement

## Contributing

Contributions are welcome! Please feel free to submit issues, fork the repository, and create pull requests.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

See LICENSE file for details.
