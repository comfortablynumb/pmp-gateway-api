# Request ID Middleware Tests
# Tests that request IDs are generated and propagated

# Test 1: Request ID is generated when not provided
GET {{base_url}}/health
HTTP 200
[Asserts]
header "X-Request-Id" exists
header "X-Request-Id" matches "^[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}$"

# Test 2: Request ID is preserved when provided
GET {{base_url}}/health
X-Request-Id: custom-request-id-12345
HTTP 200
[Asserts]
header "X-Request-Id" == "custom-request-id-12345"
