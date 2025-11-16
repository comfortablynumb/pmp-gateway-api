# Traffic Split and A/B Testing Tests
# Tests traffic splitting, canary deployments, and A/B testing

# Test 1: Request with variant selection via header
GET {{base_url}}/api/split-test
X-Variant: variant-a
HTTP *

# Test 2: Request with variant selection via cookie
GET {{base_url}}/api/split-test
Cookie: variant=variant-b
HTTP *
[Captures]
sticky_cookie: cookie "variant-sticky"

# Test 3: Sticky session maintained
GET {{base_url}}/api/split-test
Cookie: variant-sticky={{sticky_cookie}}
HTTP *

# Test 4: Query parameter based routing
GET {{base_url}}/api/split-test?variant=variant-a
HTTP *

# Test 5: Default percentage-based routing
GET {{base_url}}/api/split-test
HTTP *

# Test 6: Multiple requests for distribution testing
GET {{base_url}}/api/split-test
HTTP *

GET {{base_url}}/api/split-test
HTTP *

GET {{base_url}}/api/split-test
HTTP *

GET {{base_url}}/api/split-test
HTTP *

GET {{base_url}}/api/split-test
HTTP *

# Test 7: Canary deployment header routing
GET {{base_url}}/api/canary-test
X-Canary: true
HTTP *

# Test 8: Canary deployment cookie routing
GET {{base_url}}/api/canary-test
Cookie: canary=enabled
HTTP *
