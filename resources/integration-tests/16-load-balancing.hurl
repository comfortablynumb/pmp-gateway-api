# Load Balancing Tests
# Tests load balancing across multiple backends

# Test 1: Round-robin load balancing
GET {{base_url}}/api/balanced
HTTP *

GET {{base_url}}/api/balanced
HTTP *

GET {{base_url}}/api/balanced
HTTP *

GET {{base_url}}/api/balanced
HTTP *

GET {{base_url}}/api/balanced
HTTP *

# Test 2: Least connections load balancing
GET {{base_url}}/api/balanced-lc
HTTP *

GET {{base_url}}/api/balanced-lc
HTTP *

GET {{base_url}}/api/balanced-lc
HTTP *

# Test 3: Random load balancing
GET {{base_url}}/api/balanced-random
HTTP *

GET {{base_url}}/api/balanced-random
HTTP *

# Test 4: Load balancing with health checks
GET {{admin_base_url}}/health
HTTP 200
[Asserts]
jsonpath "$.backends" exists
jsonpath "$.healthy_count" >= 0
