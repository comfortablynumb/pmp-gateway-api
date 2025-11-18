# Response Caching Tests
# Tests response caching with cache headers

# Test 1: First request should miss cache
GET {{base_url}}/health
HTTP 200
[Asserts]
header "X-Cache" == "MISS"

# Test 2: Second identical request should hit cache
GET {{base_url}}/health
HTTP 200
[Asserts]
header "X-Cache" == "HIT"

# Test 3: Cache with query parameters
GET {{base_url}}/health?test=1
HTTP 200
[Asserts]
header "X-Cache" exists

# Test 4: Cache-Control header honored
GET {{base_url}}/health
Cache-Control: no-cache
HTTP 200
[Asserts]
header "X-Cache" == "MISS"
