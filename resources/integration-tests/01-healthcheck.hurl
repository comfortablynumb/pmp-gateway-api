# Health Check Endpoint Test
# Tests the basic health check functionality

GET {{base_url}}/health
HTTP 200
[Asserts]
jsonpath "$.status" == "healthy"
jsonpath "$.timestamp" exists
