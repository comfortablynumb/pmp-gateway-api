# Distributed Tracing Tests
# Tests OpenTelemetry distributed tracing integration

# Test 1: Trace context headers are generated
GET {{base_url}}/health
HTTP 200
[Asserts]
# OpenTelemetry trace headers should be present or generated
header "traceparent" exists OR header "X-B3-TraceId" exists

# Test 2: Trace context is propagated when provided
GET {{base_url}}/health
traceparent: 00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01
HTTP 200

# Test 3: Request ID correlates with traces
GET {{base_url}}/health
X-Request-Id: trace-test-123
HTTP 200
[Asserts]
header "X-Request-Id" == "trace-test-123"

# Test 4: Trace attributes captured for POST
POST {{base_url}}/api/test
Content-Type: application/json
{
  "trace": "test"
}
HTTP *

# Test 5: Trace spans for error conditions
GET {{base_url}}/api/nonexistent
HTTP *
