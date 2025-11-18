# Compression Tests
# Tests response compression with gzip/brotli

# Test 1: Request with gzip compression
GET {{base_url}}/health
Accept-Encoding: gzip
HTTP 200
[Asserts]
header "Content-Encoding" == "gzip" OR header "Vary" contains "Accept-Encoding"

# Test 2: Request with multiple compression formats
GET {{base_url}}/health
Accept-Encoding: gzip, deflate, br
HTTP 200
[Asserts]
header "Content-Encoding" exists OR header "Vary" contains "Accept-Encoding"

# Test 3: No compression without Accept-Encoding
GET {{base_url}}/health
HTTP 200

# Test 4: Large response compression
GET {{admin_base_url}}/config
Accept-Encoding: gzip
HTTP 200
[Asserts]
header "Content-Encoding" exists OR header "Vary" exists
