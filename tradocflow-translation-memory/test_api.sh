#!/bin/bash

# Test script for TradocFlow Translation Memory REST API

echo "🧪 Testing TradocFlow Translation Memory REST API"
echo "================================================"

BASE_URL="http://localhost:8080"

# Function to test endpoint
test_endpoint() {
    local method=$1
    local endpoint=$2
    local data=$3
    local headers=$4
    
    echo ""
    echo "🔍 Testing: $method $endpoint"
    echo "---"
    
    if [ -n "$data" ]; then
        if [ -n "$headers" ]; then
            curl -s -X $method "$BASE_URL$endpoint" \
                 -H "Content-Type: application/json" \
                 -H "$headers" \
                 -d "$data" | jq '.'
        else
            curl -s -X $method "$BASE_URL$endpoint" \
                 -H "Content-Type: application/json" \
                 -d "$data" | jq '.'
        fi
    else
        if [ -n "$headers" ]; then
            curl -s -X $method "$BASE_URL$endpoint" \
                 -H "$headers" | jq '.'
        else
            curl -s -X $method "$BASE_URL$endpoint" | jq '.'
        fi
    fi
}

echo ""
echo "📊 1. Health Check"
test_endpoint "GET" "/health"

echo ""
echo "👤 2. User Registration"
test_endpoint "POST" "/api/auth/register" '{
  "username": "testuser",
  "email": "test@example.com",
  "password": "testpass123"
}'

echo ""
echo "🔐 3. User Login"
LOGIN_RESPONSE=$(curl -s -X POST "$BASE_URL/api/auth/login" \
    -H "Content-Type: application/json" \
    -d '{
      "username": "testuser",
      "password": "testpass123"
    }')

echo "$LOGIN_RESPONSE" | jq '.'

# Extract token for authenticated requests
TOKEN=$(echo "$LOGIN_RESPONSE" | jq -r '.data.token')

if [ "$TOKEN" != "null" ] && [ -n "$TOKEN" ]; then
    echo ""
    echo "✅ Authentication successful! Token: ${TOKEN:0:20}..."
    
    echo ""
    echo "📝 4. Create Translation Unit"
    test_endpoint "POST" "/api/v1/translation-units" '{
      "source_text": "Hello world",
      "target_text": "Hola mundo",
      "source_language": "en",
      "target_language": "es",
      "context": "greeting"
    }' "Authorization: Bearer $TOKEN"
    
    echo ""
    echo "📦 5. Batch Create Translation Units"
    test_endpoint "POST" "/api/v1/translation-units/batch" '{
      "units": [
        {
          "source_text": "Good morning",
          "target_text": "Buenos días",
          "source_language": "en",
          "target_language": "es"
        },
        {
          "source_text": "Thank you",
          "target_text": "Gracias",
          "source_language": "en",
          "target_language": "es"
        }
      ]
    }' "Authorization: Bearer $TOKEN"
    
    echo ""
    echo "🔍 6. Search Translations"
    test_endpoint "GET" "/api/v1/search?q=hello&source=en&target=es&limit=5" "" "Authorization: Bearer $TOKEN"
    
    echo ""
    echo "🎯 7. Fuzzy Search"
    test_endpoint "GET" "/api/v1/search/fuzzy?q=hello&source=en&target=es&threshold=0.5" "" "Authorization: Bearer $TOKEN"
    
    echo ""
    echo "💡 8. Get Translation Suggestions"
    test_endpoint "GET" "/api/v1/suggestions?text=hello&target_language=es&limit=3" "" "Authorization: Bearer $TOKEN"
    
    echo ""
    echo "📊 9. Get Memory Statistics"
    test_endpoint "GET" "/api/v1/memories/test-memory/stats" "" "Authorization: Bearer $TOKEN"
    
else
    echo "❌ Authentication failed! Cannot test protected endpoints."
fi

echo ""
echo "📖 10. API Documentation"
test_endpoint "GET" "/api/docs"

echo ""
echo "🎉 API Testing Complete!"
echo ""
echo "To start the server manually:"
echo "  cargo run --bin rest_server"
echo ""
echo "Default endpoints:"
echo "  Health: GET $BASE_URL/health"
echo "  Docs:   GET $BASE_URL/api/docs"
echo "  Auth:   POST $BASE_URL/api/auth/register"
echo "          POST $BASE_URL/api/auth/login"