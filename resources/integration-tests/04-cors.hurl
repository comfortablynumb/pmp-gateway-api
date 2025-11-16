# CORS Tests
# Tests Cross-Origin Resource Sharing headers

# Test 1: Preflight OPTIONS request
OPTIONS {{base_url}}/health
Origin: https://example.com
Access-Control-Request-Method: GET
Access-Control-Request-Headers: Content-Type
HTTP 200
[Asserts]
header "Access-Control-Allow-Origin" exists
header "Access-Control-Allow-Methods" exists
header "Access-Control-Allow-Headers" exists

# Test 2: GET request with CORS headers
GET {{base_url}}/health
Origin: https://example.com
HTTP 200
[Asserts]
header "Access-Control-Allow-Origin" exists
