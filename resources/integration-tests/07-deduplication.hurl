# Request Deduplication Tests
# Tests idempotency and request deduplication

# Test 1: GET requests are automatically deduplicated
GET {{base_url}}/health
HTTP 200

GET {{base_url}}/health
HTTP 200
[Asserts]
header "X-Deduplicated" == "true"

# Test 2: POST with idempotency key
POST {{base_url}}/api/test
Idempotency-Key: test-key-12345
Content-Type: application/json
{
  "data": "test"
}
HTTP *

# Test 3: Duplicate POST with same idempotency key
POST {{base_url}}/api/test
Idempotency-Key: test-key-12345
Content-Type: application/json
{
  "data": "test"
}
HTTP *
[Asserts]
header "X-Deduplicated" == "true"

# Test 4: PUT requests are deduplicated
PUT {{base_url}}/api/test/123
Content-Type: application/json
{
  "data": "update"
}
HTTP *

PUT {{base_url}}/api/test/123
Content-Type: application/json
{
  "data": "update"
}
HTTP *
[Asserts]
header "X-Deduplicated" == "true"
