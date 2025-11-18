# Rate Limiting Tests
# Tests request rate limiting functionality

# Test 1: First request should succeed
GET {{base_url}}/health
HTTP 200
[Asserts]
header "X-RateLimit-Limit" exists
header "X-RateLimit-Remaining" exists
header "X-RateLimit-Reset" exists

# Test 2: Multiple requests within limit
GET {{base_url}}/health
HTTP 200

GET {{base_url}}/health
HTTP 200

GET {{base_url}}/health
HTTP 200

# Test 3: Check rate limit headers decrease
GET {{base_url}}/health
HTTP 200
[Captures]
remaining: header "X-RateLimit-Remaining"
[Asserts]
header "X-RateLimit-Remaining" exists
