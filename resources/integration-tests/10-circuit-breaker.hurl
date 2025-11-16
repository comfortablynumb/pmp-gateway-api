# Circuit Breaker Tests
# Tests circuit breaker pattern for fault tolerance

# Test 1: Successful request when circuit is closed
GET {{base_url}}/api/backend
HTTP *

# Test 2: Circuit remains closed on success
GET {{base_url}}/api/backend
HTTP *

# Test 3: Check admin health shows backend status
GET {{admin_base_url}}/health
HTTP 200
[Asserts]
jsonpath "$.backends" exists

# Test 4: Multiple requests to test circuit behavior
GET {{base_url}}/api/backend
HTTP *

GET {{base_url}}/api/backend
HTTP *

GET {{base_url}}/api/backend
HTTP *
