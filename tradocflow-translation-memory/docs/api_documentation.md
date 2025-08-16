# TradocFlow Translation Memory REST API Documentation

## Overview

The TradocFlow Translation Memory REST API provides programmatic access to translation memory functionality including:
- Translation unit management (CRUD operations)
- Fuzzy matching and similarity search
- Batch operations for bulk imports
- Translation suggestions
- User authentication and authorization

## Base URL

```
http://localhost:8080/api/v1
```

## Authentication

All API endpoints (except authentication and health check) require a JWT bearer token in the Authorization header:

```
Authorization: Bearer <your-jwt-token>
```

### Authentication Endpoints

#### Register User
```http
POST /api/auth/register
Content-Type: application/json

{
  "username": "your_username",
  "email": "your_email@example.com", 
  "password": "your_password"
}
```

#### Login User
```http
POST /api/auth/login
Content-Type: application/json

{
  "username": "your_username",
  "password": "your_password"
}
```

## Translation Unit Endpoints

### Create Translation Unit
```http
POST /api/v1/translation-units
Authorization: Bearer <token>
Content-Type: application/json

{
  "source_text": "Hello world",
  "target_text": "Hola mundo",
  "source_language": "en",
  "target_language": "es",
  "context": "greeting",
  "quality_score": 95
}
```

### Get Translation Unit
```http
GET /api/v1/translation-units/{id}
Authorization: Bearer <token>
```

### Update Translation Unit
```http
PUT /api/v1/translation-units/{id}
Authorization: Bearer <token>
Content-Type: application/json

{
  "target_text": "¡Hola mundo!",
  "quality_score": 98
}
```

### Delete Translation Unit
```http
DELETE /api/v1/translation-units/{id}
Authorization: Bearer <token>
```

### Batch Create Translation Units
```http
POST /api/v1/translation-units/batch
Authorization: Bearer <token>
Content-Type: application/json

{
  "units": [
    {
      "source_text": "Hello",
      "target_text": "Hola",
      "source_language": "en",
      "target_language": "es"
    },
    {
      "source_text": "Goodbye",
      "target_text": "Adiós",
      "source_language": "en", 
      "target_language": "es"
    }
  ]
}
```

## Search Endpoints

### Search Translations
```http
GET /api/v1/search?q=hello&source=en&target=es&threshold=0.8&limit=10
Authorization: Bearer <token>
```

### Fuzzy Search
```http
GET /api/v1/search/fuzzy?q=hello&source=en&target=es&threshold=0.5&limit=20
Authorization: Bearer <token>
```

### Get Translation Suggestions
```http
GET /api/v1/suggestions?text=hello world&target_language=es&limit=5
Authorization: Bearer <token>
```

## Memory Management

### Get Memory Statistics
```http
GET /api/v1/memories/{memory_id}/stats
Authorization: Bearer <token>
```

### Import TMX File
```http
POST /api/v1/import/tmx
Authorization: Bearer <token>
Content-Type: multipart/form-data

(TMX file upload)
```

### Export TMX File
```http
GET /api/v1/export/tmx/{memory_id}
Authorization: Bearer <token>
```

## Response Format

All API responses follow this standard format:

```json
{
  "success": true,
  "data": {
    // Response data
  },
  "error": null,
  "meta": {
    "total": 100,
    "page": 1,
    "per_page": 20,
    "total_pages": 5
  }
}
```

### Error Response
```json
{
  "success": false,
  "data": null,
  "error": "Error message",
  "meta": null
}
```

## Language Codes

The API supports standard ISO 639-1 language codes:
- `en` - English
- `es` - Spanish  
- `fr` - French
- `de` - German
- `it` - Italian
- `pt` - Portuguese
- `zh` - Chinese
- `ja` - Japanese
- `ko` - Korean
- `ru` - Russian

Custom language codes are also supported for specialized use cases.

## Rate Limiting

- 1000 requests per hour per user
- 100 requests per minute per IP address
- Bulk operations may have different limits

## Error Codes

- `400 Bad Request` - Invalid request parameters
- `401 Unauthorized` - Missing or invalid authentication
- `404 Not Found` - Resource not found
- `429 Too Many Requests` - Rate limit exceeded
- `500 Internal Server Error` - Server error

## Health Check

```http
GET /health
```

Returns server status and version information.