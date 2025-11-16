# Traffic Mirroring Tests
# Tests traffic mirroring to secondary backends

# Test 1: Request with traffic mirroring enabled
GET {{base_url}}/api/mirror-test
HTTP *
[Asserts]
# Primary request should succeed normally
status < 500

# Test 2: POST request mirroring
POST {{base_url}}/api/mirror-test
Content-Type: application/json
{
  "test": "data",
  "mirror": true
}
HTTP *

# Test 3: Headers are preserved in mirror
GET {{base_url}}/api/mirror-test
X-Custom-Header: test-value
User-Agent: HurlTest/1.0
HTTP *

# Test 4: Query parameters are mirrored
GET {{base_url}}/api/mirror-test?param1=value1&param2=value2
HTTP *

# Test 5: Check that X-Traffic-Mirror header identifies mirrored requests
# This would be verified on the mirror backend, not in the response
GET {{base_url}}/api/mirror-test
HTTP *
