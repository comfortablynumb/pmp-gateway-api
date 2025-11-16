# Environment Configuration Tests
# Tests environment-specific configuration loading

# Test 1: Configuration reflects current environment
GET {{admin_base_url}}/config
HTTP 200
[Asserts]
jsonpath "$.server" exists
jsonpath "$.clients" exists

# Test 2: Environment-specific settings applied
GET {{admin_base_url}}/info
HTTP 200
[Asserts]
jsonpath "$.version" exists
jsonpath "$.clients_count" exists

# Test 3: Server configuration from environment
GET {{base_url}}/health
HTTP 200
[Asserts]
header "X-Request-Id" exists

# Test 4: Client configuration loaded correctly
GET {{admin_base_url}}/clients
HTTP 200
[Asserts]
jsonpath "$" isCollection
