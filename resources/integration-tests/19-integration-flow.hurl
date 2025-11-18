# Integration Flow Tests
# Tests complete end-to-end flows with multiple features

# Test 1: Complete request with all middleware
GET {{base_url}}/api/complete
X-Request-Id: integration-test-001
Accept-Encoding: gzip
Origin: https://example.com
HTTP *
[Asserts]
header "X-Request-Id" == "integration-test-001"
header "X-Cache" exists
header "Access-Control-Allow-Origin" exists

# Test 2: POST request with deduplication and caching
POST {{base_url}}/api/complete
Idempotency-Key: integration-key-001
Content-Type: application/json
Accept-Encoding: gzip
{
  "test": "data",
  "timestamp": 1234567890
}
HTTP *
[Asserts]
header "X-Request-Id" exists

# Test 3: Duplicate POST should be deduplicated
POST {{base_url}}/api/complete
Idempotency-Key: integration-key-001
Content-Type: application/json
{
  "test": "data",
  "timestamp": 1234567890
}
HTTP *
[Asserts]
header "X-Deduplicated" == "true"

# Test 4: Request with traffic split and mirroring
GET {{base_url}}/api/split-and-mirror
X-Variant: variant-a
HTTP *

# Test 5: Admin check after operations
GET {{admin_base_url}}/health
HTTP 200
[Asserts]
jsonpath "$.status" exists

# Test 6: Metrics updated after all operations
GET {{base_url}}/metrics
HTTP 200
[Asserts]
body contains "http_requests_total"
