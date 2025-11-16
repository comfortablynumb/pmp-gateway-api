# Metrics Tests
# Tests Prometheus metrics endpoint

# Test 1: Metrics endpoint is accessible
GET {{base_url}}/metrics
HTTP 200
[Asserts]
header "Content-Type" contains "text/plain"
body contains "http_requests_total"
body contains "http_request_duration_seconds"

# Test 2: Metrics are updated after requests
GET {{base_url}}/health
HTTP 200

GET {{base_url}}/metrics
HTTP 200
[Asserts]
body contains "http_requests_total"
body contains "method="
body contains "path="
body contains "status="

# Test 3: Metrics include custom labels
GET {{base_url}}/metrics
HTTP 200
[Asserts]
body contains "http_requests_total{"
body contains "http_request_duration_seconds{"
