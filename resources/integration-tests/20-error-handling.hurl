# Error Handling Tests
# Tests error responses and edge cases

# Test 1: 404 for non-existent route
GET {{base_url}}/api/does-not-exist
HTTP 404
[Asserts]
header "X-Request-Id" exists

# Test 2: 405 for wrong HTTP method
POST {{base_url}}/health
HTTP 405

# Test 3: Invalid JSON payload
POST {{base_url}}/api/test
Content-Type: application/json
{invalid json}
HTTP 400

# Test 4: Missing required headers
POST {{base_url}}/api/requires-auth
Content-Type: application/json
{}
HTTP 401

# Test 5: Request body too large
POST {{base_url}}/api/test
Content-Type: application/json
```
{
  "large_field": "x".repeat(10000000)
}
```
HTTP 413

# Test 6: Unsupported media type
POST {{base_url}}/api/test
Content-Type: application/xml
<xml>test</xml>
HTTP 415

# Test 7: Rate limit exceeded (429)
# This test would need to make many requests rapidly
GET {{base_url}}/api/rate-limited
HTTP *

# Test 8: Backend timeout
GET {{base_url}}/api/slow-backend?delay=60000
HTTP 504

# Test 9: Circuit breaker open (503)
GET {{base_url}}/api/failing-backend
HTTP *
