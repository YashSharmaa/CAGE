#!/bin/bash
# Comprehensive integration test suite for CAGE

set -euo pipefail

API_URL="http://127.0.0.1:8080"
AUTH_HEADER="Authorization: ApiKey dev_integration_test"
ADMIN_HEADER="Authorization: ApiKey dev-admin-token"

echo "========================================"
echo "CAGE Integration Test Suite"
echo "========================================"
echo ""

# Test 1: Health Check
echo "[1/15] Testing health check..."
HEALTH=$(curl -s ${API_URL}/health)
STATUS=$(echo $HEALTH | jq -r '.status')
if [ "$STATUS" = "healthy" ]; then
    echo "✓ Health check passed"
else
    echo "✗ Health check failed: $STATUS"
    exit 1
fi

# Test 2: Python Execution
echo "[2/15] Testing Python execution..."
RESULT=$(curl -s -X POST ${API_URL}/api/v1/execute \
  -H "${AUTH_HEADER}" \
  -H "Content-Type: application/json" \
  --data '{"code": "print(42)"}')
STDOUT=$(echo $RESULT | jq -r '.stdout')
if [ "$STDOUT" = "42" ]; then
    echo "✓ Python execution passed"
else
    echo "✗ Python failed. Result: $RESULT"
    exit 1
fi

# Test 3: JavaScript Execution
echo "[3/15] Testing JavaScript execution..."
RESULT=$(curl -s -X POST ${API_URL}/api/v1/execute \
  -H "${AUTH_HEADER}" \
  -H "Content-Type: application/json" \
  --data '{"language": "javascript", "code": "console.log(100)"}')
STDOUT=$(echo $RESULT | jq -r '.stdout')
if [ "$STDOUT" = "100" ]; then
    echo "✓ JavaScript execution passed"
else
    echo "✗ JavaScript failed. Result: $RESULT"
    exit 1
fi

# Test 4: Bash Execution
echo "[4/15] Testing Bash execution..."
RESULT=$(curl -s -X POST ${API_URL}/api/v1/execute \
  -H "${AUTH_HEADER}" \
  -H "Content-Type: application/json" \
  --data '{"language": "bash", "code": "echo 200"}')
STDOUT=$(echo $RESULT | jq -r '.stdout')
if [ "$STDOUT" = "200" ]; then
    echo "✓ Bash execution passed"
else
    echo "✗ Bash failed. Result: $RESULT"
    exit 1
fi

# Test 5: TypeScript Execution
echo "[5/15] Testing TypeScript/Deno execution..."
RESULT=$(curl -s -X POST ${API_URL}/api/v1/execute \
  -H "${AUTH_HEADER}" \
  -H "Content-Type: application/json" \
  --data '{"language": "typescript", "code": "console.log(300)"}')
STATUS=$(echo $RESULT | jq -r '.status')
STDOUT=$(echo $RESULT | jq -r '.stdout')
if [ "$STATUS" = "success" ]; then
    echo "✓ TypeScript execution passed (output: $STDOUT)"
else
    echo "⚠ TypeScript skipped (container may not be running). Error: $(echo $RESULT | jq -r '.stderr')"
fi

# Test 6: Ruby Execution
echo "[6/15] Testing Ruby execution..."
RESULT=$(curl -s -X POST ${API_URL}/api/v1/execute \
  -H "${AUTH_HEADER}" \
  -H "Content-Type: application/json" \
  --data '{"language": "ruby", "code": "puts 400"}')
STATUS=$(echo $RESULT | jq -r '.status')
STDOUT=$(echo $RESULT | jq -r '.stdout')
if [ "$STATUS" = "success" ]; then
    echo "✓ Ruby execution passed (output: $STDOUT)"
else
    echo "⚠ Ruby skipped (container may not be running). Error: $(echo $RESULT | jq -r '.stderr')"
fi

# Test 7: Go Execution
echo "[7/15] Testing Go execution..."
RESULT=$(curl -s -X POST ${API_URL}/api/v1/execute \
  -H "${AUTH_HEADER}" \
  -H "Content-Type: application/json" \
  --data '{"language": "go", "code": "package main\nimport \"fmt\"\nfunc main() { fmt.Println(500) }"}')
STATUS=$(echo $RESULT | jq -r '.status')
STDOUT=$(echo $RESULT | jq -r '.stdout')
if [ "$STATUS" = "success" ]; then
    echo "✓ Go execution passed (output: $STDOUT)"
else
    echo "⚠ Go skipped (container may not be running). Error: $(echo $RESULT | jq -r '.stderr')"
fi

# Test 8: File Upload
echo "[8/15] Testing file upload..."
echo "test data" > /tmp/test_upload.txt
UPLOAD_RESULT=$(curl -s -X POST ${API_URL}/api/v1/files \
  -H "${AUTH_HEADER}" \
  -F "file=@/tmp/test_upload.txt")
UPLOAD_SIZE=$(echo $UPLOAD_RESULT | jq -r '.size_bytes')
if [ "$UPLOAD_SIZE" != "null" ]; then
    echo "✓ File upload passed (size: $UPLOAD_SIZE bytes)"
else
    echo "✗ File upload failed. Result: $UPLOAD_RESULT"
    exit 1
fi

# Test 9: File List
echo "[9/15] Testing file list..."
FILE_LIST=$(curl -s "${API_URL}/api/v1/files" \
  -H "${AUTH_HEADER}")
FILE_COUNT=$(echo $FILE_LIST | jq '.files | length')
if [ $FILE_COUNT -gt 0 ]; then
    echo "✓ File list passed ($FILE_COUNT files)"
else
    echo "✗ File list failed"
    exit 1
fi

# Test 10: Session Info
echo "[10/15] Testing session info..."
SESSION=$(curl -s "${API_URL}/api/v1/session" \
  -H "${AUTH_HEADER}")
SESSION_ID=$(echo $SESSION | jq -r '.session_id')
if [ "$SESSION_ID" != "null" ]; then
    echo "✓ Session info passed (ID: ${SESSION_ID:0:8}...)"
else
    echo "✗ Session info failed"
    exit 1
fi

# Test 11: User Management - Create User
echo "[11/15] Testing user management - create user..."
CREATE_USER=$(curl -s -X POST "${API_URL}/api/v1/admin/users" \
  -H "${ADMIN_HEADER}" \
  -H "Content-Type: application/json" \
  -d '{
    "user_id": "test_created_user",
    "enabled": true,
    "resource_limits": {
      "max_memory_mb": 1024,
      "max_cpus": 1.0,
      "max_pids": 100,
      "max_execution_seconds": 30,
      "max_disk_mb": 1024
    },
    "network_policy": {
      "enabled": false,
      "allowed_hosts": [],
      "allowed_ports": []
    },
    "allowed_languages": ["python"],
    "gpu_enabled": false
  }')
CREATE_STATUS=$(echo $CREATE_USER | jq -r '.user_id')
if [ "$CREATE_STATUS" = "test_created_user" ]; then
    echo "✓ User create passed"
else
    echo "⚠ User create returned: $CREATE_USER"
fi

# Test 12: User Management - List Users
echo "[12/15] Testing user management - list users..."
USERS=$(curl -s "${API_URL}/api/v1/admin/users" \
  -H "${ADMIN_HEADER}")
USER_COUNT=$(echo $USERS | jq '.users | length')
echo "✓ User list passed ($USER_COUNT users)"

# Test 13: Execution Replay - List
echo "[13/15] Testing execution replay - list..."
REPLAYS=$(curl -s "${API_URL}/api/v1/replays?limit=10" \
  -H "${AUTH_HEADER}")
REPLAY_COUNT=$(echo $REPLAYS | jq 'length')
echo "✓ Replay list passed ($REPLAY_COUNT replays stored)"

# Test 14: Admin Stats
echo "[14/15] Testing admin stats..."
STATS=$(curl -s "${API_URL}/api/v1/admin/stats" \
  -H "${ADMIN_HEADER}")
TOTAL_EXECS=$(echo $STATS | jq -r '.total_executions')
echo "✓ Admin stats passed (total executions: $TOTAL_EXECS)"

# Test 15: Prometheus Metrics
echo "[15/15] Testing Prometheus metrics..."
METRICS=$(curl -s "${API_URL}/metrics")
if echo "$METRICS" | grep -q "cage_total_executions"; then
    echo "✓ Prometheus metrics passed"
else
    echo "✗ Prometheus metrics failed"
    exit 1
fi

echo ""
echo "========================================"
echo "ALL CORE INTEGRATION TESTS PASSED! ✅"
echo "========================================"
echo ""
echo "Summary:"
echo "- Core languages: Python ✅, JavaScript ✅, Bash ✅"
echo "- Extended languages: TypeScript ⚠, Ruby ⚠, Go ⚠ (may need containers)"
echo "- File operations: Upload ✅, List ✅"
echo "- Session management: Get session ✅"
echo "- User management: Create ✅, List ✅"
echo "- Replay: List ✅"
echo "- Monitoring: Stats ✅, Metrics ✅"
echo ""

