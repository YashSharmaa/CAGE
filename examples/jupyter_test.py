#!/usr/bin/env python3
"""
Test persistent interpreter mode with state retention
"""

import requests
import time

API_URL = "http://localhost:8080/api/v1"
AUTH_HEADER = {"Authorization": "ApiKey dev_jupyter_test"}

def execute(code, persistent=False):
    """Execute code and return result"""
    response = requests.post(
        f"{API_URL}/execute",
        json={
            "code": code,
            "persistent": persistent
        },
        headers=AUTH_HEADER
    )
    return response.json()

def main():
    print("=== Testing Persistent Interpreter Mode ===\n")

    # Test 1: Ephemeral mode (no state retention)
    print("Test 1: Ephemeral Mode (default)")
    print("  Setting variable...")
    result1 = execute("x = 100")
    print(f"  Status: {result1['status']}")

    print("  Trying to access variable...")
    result2 = execute("print(x)")
    print(f"  Status: {result2['status']}")
    print(f"  Output: {result2.get('stderr', 'No error')}")
    if "NameError" in result2.get('stderr', ''):
        print("  ✓ Variable NOT retained (correct for ephemeral mode)")

    print("\n" + "="*50 + "\n")

    # Test 2: Persistent mode (state retention)
    print("Test 2: Persistent Mode")
    print("  Setting variable x=42...")
    result3 = execute("x = 42; print(f'Set x={x}')", persistent=True)
    print(f"  Output: {result3['stdout']}")

    print("  Importing library...")
    result4 = execute("import pandas as pd; print('Pandas imported')", persistent=True)
    print(f"  Output: {result4['stdout']}")

    print("  Using previous variable and import...")
    result5 = execute("""
df = pd.DataFrame({'value': [x, x*2, x*3]})
print(df)
print(f'x still equals {x}')
""", persistent=True)
    print(f"  Output:\n{result5['stdout']}")

    if result5['status'] == 'success':
        print("\n  ✓ State RETAINED across executions (persistent mode working!)")

    print("\n" + "="*50 + "\n")

    # Test 3: State isolation between users
    print("Test 3: User Isolation")
    # This would need a different API key/user
    print("  (Requires separate user - skipped in demo)")

if __name__ == "__main__":
    main()
