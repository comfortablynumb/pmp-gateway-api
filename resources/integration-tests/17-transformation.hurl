# Request/Response Transformation Tests
# Tests field mapping, filtering, and transformations

# Test 1: Response field mapping
GET {{base_url}}/api/transform/mapping
HTTP *
[Asserts]
jsonpath "$.transformed" exists

# Test 2: Response field filtering (include)
GET {{base_url}}/api/transform/filter-include
HTTP *
[Asserts]
jsonpath "$" exists

# Test 3: Response field filtering (exclude)
GET {{base_url}}/api/transform/filter-exclude
HTTP *
[Asserts]
jsonpath "$" exists

# Test 4: Request interpolation with path parameters
GET {{base_url}}/api/users/123/profile
HTTP *

# Test 5: Request interpolation with query parameters
GET {{base_url}}/api/search?q=test&limit=10
HTTP *

# Test 6: Request interpolation with headers
GET {{base_url}}/api/test
X-Custom-Header: custom-value
Authorization: Bearer token123
HTTP *

# Test 7: Complex transformation with multiple mappings
POST {{base_url}}/api/transform/complex
Content-Type: application/json
{
  "input_field": "test",
  "nested": {
    "value": 123
  }
}
HTTP *
