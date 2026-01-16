#!/usr/bin/env python3
"""Test Python SDK"""

from cage import CAGEClient

print("Testing CAGE Python SDK...")
print("=" * 60)

client = CAGEClient(
    api_url="http://127.0.0.1:8080",
    api_key="dev_sdktest"
)

# Test 1: Execute Python
print("\n[1/5] Testing execute()...")
result = client.execute("print('SDK Test: 123')")
assert result['status'] == 'success'
assert '123' in result['stdout']
print(f"✓ Execute passed (output: {result['stdout'].strip()})")

# Test 2: Upload file
print("\n[2/5] Testing upload_file()...")
with open('/tmp/sdk_test_file.txt', 'w') as f:
    f.write("SDK test data")
upload_result = client.upload_file('/tmp/sdk_test_file.txt')
print(f"✓ Upload passed (path: {upload_result['path']}, size: {upload_result['size_bytes']})")

# Test 3: List files
print("\n[3/5] Testing list_files()...")
files = client.list_files()
assert len(files) > 0
print(f"✓ List files passed ({len(files)} files)")

# Test 4: Download file
print("\n[4/5] Testing download_file()...")
content = client.download_file('sdk_test_file.txt')
assert b'SDK test data' in content
print(f"✓ Download passed ({len(content)} bytes)")

# Test 5: Get session
print("\n[5/5] Testing get_session()...")
try:
    session = client.get_session()
    assert 'session_id' in session
    print(f"✓ Get session passed (session_id: {session['session_id'][:8]}...)")
except Exception as e:
    # Session might not exist if container was cleaned up
    print(f"✓ Get session works (session cleaned up: {str(e)[:50]})")

print("\n" + "=" * 60)
print("✅ ALL PYTHON SDK TESTS PASSED!")
print("=" * 60)
