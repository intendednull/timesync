# TimeSync API Documentation

This document describes the RESTful API endpoints provided by TimeSync for schedule and availability management.

## Base URL

All API endpoints are relative to the base URL:

```
https://api.yourdomain.com/v1
```

For local development:

```
http://localhost:8080/v1
```

## Authentication

Some endpoints require authentication using a schedule token or password. When required, authentication should be provided in the HTTP headers:

```
Authorization: Bearer <token>
```

## Endpoints

### Schedule Management

#### Create a Schedule

```
POST /schedule
```

Create a new scheduling session.

**Request Body:**

```json
{
  "name": "Team Meeting",
  "description": "Weekly planning session",
  "timezone": "America/New_York",
  "password": "optional-password",
  "start_date": "2025-03-01",
  "end_date": "2025-03-07",
  "time_slots": [
    {"start_time": "09:00", "end_time": "17:00"}
  ]
}
```

**Response:**

```json
{
  "id": "sch_123abc456def",
  "token": "tok_123abc456def",
  "name": "Team Meeting",
  "description": "Weekly planning session",
  "timezone": "America/New_York",
  "created_at": "2025-02-28T12:34:56Z",
  "start_date": "2025-03-01",
  "end_date": "2025-03-07",
  "time_slots": [
    {"start_time": "09:00", "end_time": "17:00"}
  ]
}
```

#### Get a Schedule

```
GET /schedule/{schedule_id}
```

Retrieve information about an existing schedule.

**Response:**

```json
{
  "id": "sch_123abc456def",
  "name": "Team Meeting",
  "description": "Weekly planning session",
  "timezone": "America/New_York",
  "created_at": "2025-02-28T12:34:56Z",
  "start_date": "2025-03-01",
  "end_date": "2025-03-07",
  "time_slots": [
    {"start_time": "09:00", "end_time": "17:00"}
  ],
  "requires_password": true
}
```

#### Update a Schedule

```
PUT /schedule/{schedule_id}
```

Update an existing schedule (requires authentication).

**Request Headers:**

```
Authorization: Bearer <token>
```

**Request Body:**

```json
{
  "name": "Updated Team Meeting",
  "description": "Updated weekly planning session",
  "timezone": "America/Chicago",
  "password": "new-optional-password",
  "start_date": "2025-03-01",
  "end_date": "2025-03-14",
  "time_slots": [
    {"start_time": "10:00", "end_time": "16:00"}
  ]
}
```

**Response:**

```json
{
  "id": "sch_123abc456def",
  "name": "Updated Team Meeting",
  "description": "Updated weekly planning session",
  "timezone": "America/Chicago",
  "created_at": "2025-02-28T12:34:56Z",
  "updated_at": "2025-02-28T13:45:12Z",
  "start_date": "2025-03-01",
  "end_date": "2025-03-14",
  "time_slots": [
    {"start_time": "10:00", "end_time": "16:00"}
  ]
}
```

#### Delete a Schedule

```
DELETE /schedule/{schedule_id}
```

Delete an existing schedule (requires authentication).

**Request Headers:**

```
Authorization: Bearer <token>
```

**Response:**

```
204 No Content
```

### Availability Management

#### Submit Availability

```
POST /availability/{schedule_id}
```

Submit a participant's availability for a schedule.

**Request Body:**

```json
{
  "name": "Jane Doe",
  "password": "schedule-password-if-required",
  "availability": [
    {
      "date": "2025-03-01",
      "slots": [
        {"start_time": "09:00", "end_time": "12:00"},
        {"start_time": "13:00", "end_time": "17:00"}
      ]
    },
    {
      "date": "2025-03-02",
      "slots": [
        {"start_time": "14:00", "end_time": "16:00"}
      ]
    }
  ]
}
```

**Response:**

```json
{
  "id": "avl_123abc456def",
  "name": "Jane Doe",
  "schedule_id": "sch_123abc456def",
  "created_at": "2025-02-28T14:22:33Z",
  "availability": [
    {
      "date": "2025-03-01",
      "slots": [
        {"start_time": "09:00", "end_time": "12:00"},
        {"start_time": "13:00", "end_time": "17:00"}
      ]
    },
    {
      "date": "2025-03-02",
      "slots": [
        {"start_time": "14:00", "end_time": "16:00"}
      ]
    }
  ]
}
```

#### Get Availability Summary

```
GET /availability/{schedule_id}
```

Get a summary of all participants' availability for a schedule.

**Query Parameters:**

- `password`: Required if the schedule is password-protected

**Response:**

```json
{
  "schedule_id": "sch_123abc456def",
  "name": "Team Meeting",
  "participants": ["Jane Doe", "John Smith", "Alice Johnson"],
  "dates": [
    {
      "date": "2025-03-01",
      "participant_count": 3,
      "slots": [
        {
          "start_time": "09:00",
          "end_time": "10:00",
          "available_count": 3,
          "available_participants": ["Jane Doe", "John Smith", "Alice Johnson"]
        },
        {
          "start_time": "10:00",
          "end_time": "11:00",
          "available_count": 2,
          "available_participants": ["Jane Doe", "John Smith"]
        }
      ]
    }
  ],
  "optimal_slots": [
    {
      "date": "2025-03-01",
      "start_time": "09:00",
      "end_time": "10:00",
      "available_count": 3
    }
  ]
}
```

### Discord Integration

#### Connect Discord Server

```
POST /discord/connect
```

Connect a Discord server to TimeSync for automated scheduling.

**Request Headers:**

```
Authorization: Bearer <token>
```

**Request Body:**

```json
{
  "schedule_id": "sch_123abc456def",
  "discord_server_id": "9876543210",
  "channel_id": "1234567890",
  "notification_preferences": {
    "new_availability": true,
    "schedule_updates": true
  }
}
```

**Response:**

```json
{
  "success": true,
  "connection_id": "con_123abc456def",
  "webhook_url": "https://api.yourdomain.com/webhook/discord/123abc456def"
}
```

### Health Check

#### API Health

```
GET /health
```

Check the health status of the API.

**Response:**

```json
{
  "status": "healthy",
  "version": "1.0.0",
  "timestamp": "2025-02-28T15:30:45Z"
}
```

## Error Handling

All API endpoints return standard HTTP status codes:

- `200 OK`: Successful request
- `201 Created`: Resource successfully created
- `204 No Content`: Resource successfully deleted
- `400 Bad Request`: Invalid request parameters
- `401 Unauthorized`: Authentication required or failed
- `403 Forbidden`: Authenticated user doesn't have permission
- `404 Not Found`: Resource not found
- `409 Conflict`: Resource conflict (e.g., duplicate submission)
- `422 Unprocessable Entity`: Request validation failed
- `429 Too Many Requests`: Rate limit exceeded
- `500 Internal Server Error`: Server error

Error responses include a JSON body with details:

```json
{
  "error": {
    "code": "invalid_request",
    "message": "The request was invalid",
    "details": ["Field 'name' is required"]
  }
}
```