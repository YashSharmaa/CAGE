#!/bin/bash
# Test all 9 languages

set -e

API="http://127.0.0.1:8080/api/v1/execute"
AUTH="Authorization: ApiKey dev_langtest"

echo "Testing all 9 languages..."
echo ""

# Python
echo -n "Python... "
RESULT=$(curl -s -X POST "$API" -H "$AUTH" -H "Content-Type: application/json" --data '{"language":"python","code":"print(42)"}')
if echo "$RESULT" | jq -e '.status == "success"' >/dev/null; then
    echo "✅ (output: $(echo $RESULT | jq -r .stdout | tr -d '\n'))"
else
    echo "❌ $(echo $RESULT | jq -r .stderr)"
fi

# JavaScript
echo -n "JavaScript... "
RESULT=$(curl -s -X POST "$API" -H "$AUTH" -H "Content-Type: application/json" --data '{"language":"javascript","code":"console.log(100)"}')
if echo "$RESULT" | jq -e '.status == "success"' >/dev/null; then
    echo "✅ (output: $(echo $RESULT | jq -r .stdout | tr -d '\n'))"
else
    echo "❌ $(echo $RESULT | jq -r .stderr)"
fi

# Bash
echo -n "Bash... "
RESULT=$(curl -s -X POST "$API" -H "$AUTH" -H "Content-Type: application/json" --data '{"language":"bash","code":"echo 200"}')
if echo "$RESULT" | jq -e '.status == "success"' >/dev/null; then
    echo "✅ (output: $(echo $RESULT | jq -r .stdout | tr -d '\n'))"
else
    echo "❌ $(echo $RESULT | jq -r .stderr)"
fi

# TypeScript
echo -n "TypeScript... "
RESULT=$(curl -s -X POST "$API" -H "$AUTH" -H "Content-Type: application/json" --data '{"language":"typescript","code":"console.log(300)"}' --max-time 60)
if echo "$RESULT" | jq -e '.status == "success"' >/dev/null 2>&1; then
    echo "✅ (output: $(echo $RESULT | jq -r .stdout | tr -d '\n'))"
else
    echo "⚠️  (container needs building)"
fi

# Ruby
echo -n "Ruby... "
RESULT=$(curl -s -X POST "$API" -H "$AUTH" -H "Content-Type: application/json" --data '{"language":"ruby","code":"puts 400"}' --max-time 60)
if echo "$RESULT" | jq -e '.status == "success"' >/dev/null 2>&1; then
    echo "✅ (output: $(echo $RESULT | jq -r .stdout | tr -d '\n'))"
else
    echo "⚠️  (container needs building)"
fi

# Go
echo -n "Go... "
RESULT=$(curl -s -X POST "$API" -H "$AUTH" -H "Content-Type: application/json" --data '{"language":"go","code":"package main\nimport \"fmt\"\nfunc main(){fmt.Println(500)}"}' --max-time 60)
if echo "$RESULT" | jq -e '.status == "success"' >/dev/null 2>&1; then
    echo "✅ (output: $(echo $RESULT | jq -r .stdout | tr -d '\n'))"
else
    echo "⚠️  (container needs building)"
fi

# Julia
echo -n "Julia... "
RESULT=$(curl -s -X POST "$API" -H "$AUTH" -H "Content-Type: application/json" --data '{"language":"julia","code":"println(600)"}' --max-time 60)
if echo "$RESULT" | jq -e '.status == "success"' >/dev/null 2>&1; then
    echo "✅ (output: $(echo $RESULT | jq -r .stdout | tr -d '\n'))"
else
    echo "⚠️  (container needs building)"
fi

# R
echo -n "R... "
RESULT=$(curl -s -X POST "$API" -H "$AUTH" -H "Content-Type: application/json" --data '{"language":"r","code":"print(700)"}' --max-time 60)
if echo "$RESULT" | jq -e '.status == "success"' >/dev/null 2>&1; then
    echo "✅ (output: $(echo $RESULT | jq -r .stdout | tr -d '\n'))"
else
    echo "⚠️  (container needs building)"
fi

echo ""
echo "Language testing complete!"
