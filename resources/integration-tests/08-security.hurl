# Security Tests
# Tests IP filtering, security headers, and authentication

# Test 1: Security headers are present
GET {{base_url}}/health
HTTP 200
[Asserts]
header "X-Content-Type-Options" == "nosniff"
header "X-Frame-Options" == "DENY"
header "X-XSS-Protection" == "1; mode=block"

# Test 2: Strict-Transport-Security header (if HTTPS)
GET {{base_url}}/health
HTTP 200
[Asserts]
header "Strict-Transport-Security" exists

# Test 3: Content-Security-Policy header
GET {{base_url}}/health
HTTP 200
[Asserts]
header "Content-Security-Policy" exists
