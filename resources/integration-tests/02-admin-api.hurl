# Admin API Tests
# Tests all administrative endpoints

# Test 1: Get gateway info
GET {{admin_base_url}}/info
HTTP 200
[Asserts]
jsonpath "$.name" == "PMP Gateway"
jsonpath "$.version" exists
jsonpath "$.clients_count" exists
jsonpath "$.routes_count" exists

# Test 2: Get aggregated health status
GET {{admin_base_url}}/health
HTTP 200
[Asserts]
jsonpath "$.status" exists
jsonpath "$.total_backends" exists
jsonpath "$.healthy_count" exists
jsonpath "$.unhealthy_count" exists

# Test 3: Get current configuration
GET {{admin_base_url}}/config
HTTP 200
[Asserts]
jsonpath "$.clients" exists
jsonpath "$.routes" exists
jsonpath "$.server" exists

# Test 4: List all routes
GET {{admin_base_url}}/routes
HTTP 200
[Asserts]
jsonpath "$" isCollection
jsonpath "$[*].method" exists
jsonpath "$[*].path" exists

# Test 5: List all clients
GET {{admin_base_url}}/clients
HTTP 200
[Asserts]
jsonpath "$" isCollection

# Test 6: Reload configuration
POST {{admin_base_url}}/config/reload
HTTP 200
[Asserts]
jsonpath "$.success" exists
jsonpath "$.message" exists
