# WebSocket Tests
# Tests WebSocket upgrade and proxying (limited by HTTP-only testing)

# Test 1: WebSocket upgrade request
GET {{base_url}}/ws
Upgrade: websocket
Connection: Upgrade
Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==
Sec-WebSocket-Version: 13
HTTP 101
[Asserts]
header "Upgrade" == "websocket"
header "Connection" contains "Upgrade"
header "Sec-WebSocket-Accept" exists

# Test 2: WebSocket endpoint without upgrade headers should fail
GET {{base_url}}/ws
HTTP 400

# Test 3: WebSocket with custom subprotocol
GET {{base_url}}/ws
Upgrade: websocket
Connection: Upgrade
Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==
Sec-WebSocket-Version: 13
Sec-WebSocket-Protocol: chat
HTTP 101
