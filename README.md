# PMP Gateway API

PMP Gateway API: A flexible, YAML-configurable API gateway that allows you to configure endpoints to proxy, aggregate, and transform requests to multiple backend services. Part of Poor Man's Platform ecosystem.

## Features

- **YAML-based Configuration**: Define routes, clients, and request mappings entirely in YAML
- **Multiple Client Types**: Support for HTTP clients (with future support for SQL, NoSQL, Redis, etc.)
- **Connection Pooling**: Efficient connection management with configurable pool sizes
- **Request Interpolation**: Dynamic request mapping using template syntax like `${request.headers["Authorization"]}`
- **Subrequests**: Execute multiple backend requests for a single incoming request
- **Header Forwarding**: Pass through or transform headers from incoming requests
- **Query Parameter Mapping**: Map and transform query parameters
- **Body Templating**: Template request bodies with data from incoming requests

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

Create a `config.yaml` file (see `config.example.yaml` for a comprehensive example):

```yaml
clients:
  my_api:
    type: http
    base_url: "https://api.example.com"
    headers:
      User-Agent: "PMP-Gateway/1.0"
    min_connections: 2
    max_connections: 20
    timeout: 30

routes:
  - method: GET
    path: /users/:id
    subrequests:
      - client_id: my_api
        type: http
        uri: /users/${request.path.id}
        method: GET
        headers:
          Authorization: "${request.headers[\"authorization\"]}"
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

## Configuration Reference

### Clients

Clients represent backend services or data sources. Each client has a unique ID used to reference it in routes.

#### HTTP Client

```yaml
clients:
  client_id:
    type: http
    base_url: "https://api.example.com"  # Base URL for all requests
    headers:                              # Default headers for all requests
      User-Agent: "My-App/1.0"
    min_connections: 1                    # Minimum connection pool size (default: 1)
    max_connections: 10                   # Maximum connection pool size (default: 10)
    timeout: 30                           # Request timeout in seconds (default: 30)
```

### Routes

Routes define the endpoints your gateway will expose.

```yaml
routes:
  - method: GET                           # HTTP method (GET, POST, PUT, DELETE, etc.)
    path: /api/users/:id                  # Path with optional parameters (:id)
    subrequests:                          # List of backend requests to execute
      - client_id: my_client              # Reference to a client ID
        type: http                        # Subrequest type (must match client type)
        uri: /users/${request.path.id}    # URI to append to client's base_url
        method: GET                       # HTTP method for this subrequest
        headers:                          # Additional headers (supports interpolation)
          Authorization: "${request.headers[\"authorization\"]}"
        body: "${request.body}"           # Request body (supports interpolation)
        query_params:                     # Query parameters (supports interpolation)
          filter: "${request.query.filter}"
```

### Interpolation Syntax

Use `${...}` syntax to reference data from the incoming request:

- **Headers**: `${request.headers["Header-Name"]}`
- **Path Parameters**: `${request.path.param_name}`
- **Query Parameters**: `${request.query.param_name}`
- **Request Body**: `${request.body}`
- **HTTP Method**: `${request.method}`

#### Examples

```yaml
# Forward authorization header
headers:
  Authorization: "${request.headers[\"authorization\"]}"

# Use path parameter in URI
uri: /users/${request.path.user_id}/posts/${request.path.post_id}

# Map query parameters
query_params:
  search: "${request.query.q}"
  limit: "${request.query.max_results}"

# Template request body
body: '{"user_id": "${request.path.id}", "action": "update"}'
```

## Use Cases

### 1. Simple Proxy

Forward requests to a backend API:

```yaml
routes:
  - method: GET
    path: /api/:resource
    subrequests:
      - client_id: backend_api
        type: http
        uri: /${request.path.resource}
        method: GET
```

### 2. Request Aggregation

Fetch data from multiple sources in a single request:

```yaml
routes:
  - method: GET
    path: /dashboard/:user_id
    subrequests:
      - client_id: user_service
        type: http
        uri: /users/${request.path.user_id}
      - client_id: analytics_service
        type: http
        uri: /stats/${request.path.user_id}
      - client_id: notifications_service
        type: http
        uri: /notifications?user=${request.path.user_id}
```

### 3. Request Transformation

Transform incoming requests before forwarding:

```yaml
routes:
  - method: POST
    path: /legacy/users
    subrequests:
      - client_id: new_api
        type: http
        uri: /v2/users
        method: POST
        headers:
          Content-Type: "application/json"
          X-API-Version: "2.0"
        body: '{"user": ${request.body}}'
```

## Environment Variables

- `CONFIG_PATH`: Path to configuration file (default: `config.yaml`)
- `HOST`: Server host (default: `0.0.0.0`)
- `PORT`: Server port (default: `3000`)
- `RUST_LOG`: Logging level (e.g., `debug`, `info`, `warn`, `error`)

## Development

### Running Tests

```bash
cargo test
```

### Running with Debug Logging

```bash
RUST_LOG=debug cargo run
```

## Future Enhancements

- [ ] SQL database clients
- [ ] NoSQL database clients (MongoDB, Cassandra)
- [ ] Redis/cache clients
- [ ] Response transformation and filtering
- [ ] Conditional subrequest execution
- [ ] Rate limiting and throttling
- [ ] Circuit breaker pattern
- [ ] Request/response caching
- [ ] Metrics and monitoring
- [ ] GraphQL support

## License

See LICENSE file for details.
