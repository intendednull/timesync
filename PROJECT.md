# Scheduling Service Project

## Project Summary

This project aims to develop a simple, anonymous scheduling service that enables groups to coordinate their availability and find common free time slots. Users can create availability schedules with unique IDs, share these IDs with others, and visualize overlapping availability to facilitate event planning without requiring user accounts or authentication. A Discord bot provides seamless integration for Discord communities.

## Core Features

### Availability Creation and Management
- Anonymous availability creation with unique ID generation
- Optional password protection for future editing
- Schedule naming for easy identification
- Read-only mode for schedules without passwords
- Simple, straightforward interface for setting available times

### Availability Visualization
- Clean, minimal calendar view showing overlapping free times
- Easy-to-read visual indication of common availability
- Simple day and week views optimized for quick comprehension
- No flashy components - focus on clarity and readability

### Discord Bot Integration
- Commands to create new availability schedules by redirecting to web UI
- Automatic schedule tracking for Discord users
- Group creation based on Discord users
- Match functionality to find common availability between groups
- Confirmation system using reactions

## Technical Architecture

### Frontend
- **Approach**: Static site with minimal JavaScript for interactivity
- **Framework**: Plain HTML/CSS with targeted JavaScript where needed
- **UI Philosophy**: Simplicity and clarity over visual complexity
- **Data Visualization**: Basic, easy-to-read calendar representations
- **Time Selection**: Interactive time grid with click-and-drag functionality

### Backend
- **Language**: Rust
- **Web Framework**: Axum
- **Runtime**: Tokio async runtime
- **API Architecture**: RESTful API design
- **Authentication**: Simple password-based for schedule editing

### Database
- **Database**: PostgreSQL
- **ORM**: SQLx for type-safe SQL queries

### Discord Bot
- **Language**: Rust
- **Framework**: Serenity for Discord API integration
- **Integration**: Direct communication with the web server's API

## Database Schema

### Tables

#### schedules
```sql
CREATE TABLE IF NOT EXISTS schedules (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL,
    password_hash VARCHAR(255) NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);
```

#### time_slots
```sql
CREATE TABLE IF NOT EXISTS time_slots (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    schedule_id UUID NOT NULL REFERENCES schedules(id),
    start_time TIMESTAMP WITH TIME ZONE NOT NULL,
    end_time TIMESTAMP WITH TIME ZONE NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    CONSTRAINT valid_time_range CHECK (end_time > start_time)
);
```

#### discord_users
```sql
CREATE TABLE IF NOT EXISTS discord_users (
    discord_id VARCHAR(255) PRIMARY KEY,
    schedule_id UUID REFERENCES schedules(id),
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);
```

#### discord_groups
```sql
CREATE TABLE IF NOT EXISTS discord_groups (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL,
    server_id VARCHAR(255) NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);
```

#### group_members
```sql
CREATE TABLE IF NOT EXISTS group_members (
    group_id UUID NOT NULL REFERENCES discord_groups(id),
    discord_id VARCHAR(255) NOT NULL REFERENCES discord_users(discord_id),
    PRIMARY KEY (group_id, discord_id)
);
```

### Indexes
```sql
CREATE INDEX idx_time_slots_schedule_id ON time_slots(schedule_id);
CREATE INDEX idx_time_slots_start_time ON time_slots(start_time);
CREATE INDEX idx_time_slots_end_time ON time_slots(end_time);
CREATE INDEX idx_discord_users_schedule_id ON discord_users(schedule_id);
CREATE INDEX idx_group_members_group_id ON group_members(group_id);
CREATE INDEX idx_group_members_discord_id ON group_members(discord_id);
CREATE INDEX idx_discord_groups_server_id ON discord_groups(server_id);
```

## REST API Endpoints

### Schedule Management

- `POST /api/schedules`
  - Creates a new availability schedule for a single user
  - Request body:
    ```json
    {
      "name": "string",
      "password": "string" (optional),
      "slots": [
        { "start": "timestamp", "end": "timestamp" }
      ],
      "discord_id": "string" (optional)
    }
    ```
  - Returns: `{ "id": "uuid", "name": "string", "created_at": "timestamp", "is_editable": boolean }`

- `GET /api/schedules/{id}`
  - Gets a single user's schedule information
  - Returns:
    ```json
    {
      "id": "uuid",
      "name": "string",
      "created_at": "timestamp",
      "is_editable": boolean,
      "slots": [
        { "start": "timestamp", "end": "timestamp" }
      ]
    }
    ```

- `PUT /api/schedules/{id}`
  - Updates a user's schedule details and availability (requires password if protected)
  - Request body:
    ```json
    {
      "name": "string" (optional),
      "slots": [
        { "start": "timestamp", "end": "timestamp" }
      ],
      "password": "string" (if protected)
    }
    ```
  - Returns: `{ "id": "uuid", "updated_at": "timestamp" }`

### Discord User Management

- `POST /api/discord/users`
  - Associates a Discord user with a schedule
  - Request body:
    ```json
    {
      "discord_id": "string",
      "schedule_id": "uuid"
    }
    ```
  - Returns: `{ "discord_id": "string", "schedule_id": "uuid" }`

- `GET /api/discord/users/{discord_id}`
  - Gets a Discord user's schedule
  - Returns:
    ```json
    {
      "discord_id": "string",
      "schedule_id": "uuid"
    }
    ```

### Discord Group Management

- `POST /api/discord/groups`
  - Creates a new Discord group
  - Request body:
    ```json
    {
      "name": "string",
      "server_id": "string",
      "member_ids": ["string"]
    }
    ```
  - Returns: `{ "id": "uuid", "name": "string", "server_id": "string" }`

- `GET /api/discord/groups/{id}`
  - Gets a Discord group's information
  - Returns:
    ```json
    {
      "id": "uuid",
      "name": "string",
      "server_id": "string",
      "members": [
        { "discord_id": "string", "schedule_id": "uuid" (if available) }
      ]
    }
    ```

- `PUT /api/discord/groups/{id}`
  - Updates a Discord group
  - Request body:
    ```json
    {
      "name": "string" (optional),
      "add_member_ids": ["string"] (optional),
      "remove_member_ids": ["string"] (optional)
    }
    ```
  - Returns: `{ "id": "uuid", "updated_at": "timestamp" }`

### Availability Analysis

- `GET /api/availability/match`
  - Gets the best meeting times for multiple groups with minimum attendance requirements
  - Query params:
    - `group_ids` (comma-separated list of group IDs)
    - `min_per_group` (optional, minimum number of users required from each group)
    - `count` (optional, number of suggestions to return)
  - Returns:
    ```json
    {
      "matches": [
        {
          "start": "timestamp",
          "end": "timestamp",
          "groups": [
            {
              "id": "uuid",
              "name": "string",
              "available_users": ["string"],
              "count": number
            }
          ]
        }
      ]
    }
    ```

### Authentication

- `POST /api/schedules/{id}/verify`
  - Verifies a schedule password
  - Request body: `{ "password": "string" }`
  - Returns: `{ "valid": boolean }`

### Utility Endpoints

- `GET /api/health`
  - Simple health check endpoint
  - Returns: `{ "status": "ok" }`

- `GET /api/version`
  - Gets the API version
  - Returns: `{ "version": "string" }`

## Frontend Routing

```
/                       # Create new schedule (homepage)
/:id                    # View a specific schedule
/:id/edit               # Edit a specific schedule (password protected)
/availability           # Compare multiple schedules
/availability?ids=...   # Pre-populated comparison with IDs
```

## User Interface Components

### Time Grid Component
- Grid layout with days on one axis and 15-minute time intervals on the other
- Click and drag functionality to select multiple time slots at once
- Visual highlighting of selected areas during drag operation
- Ability to select across multiple days
- Option to deselect areas by dragging over already selected slots
- Clear visual distinction between selected and unselected slots

### Availability Visualization Component
- Grid-based calendar view
- Color intensity indicating number of available users
- Hover tooltips showing exactly who is available
- Simple toggles for day/week view
- Responsive design that works well on all devices

## Discord Bot Commands

- `!schedule create` - Generates a unique link to the web UI for creating a schedule and associates it with the Discord user
- `!schedule group [group_name] @user1 @user2 @user3...` - Creates a new group with the specified users
- `!schedule match [group_name1] [group_name2]...` - Finds the best meeting times where members from each group can attend and pings users for confirmation through reactions

## Development Phases

### Phase 1: Core Backend
- Set up Rust project with Axum and Tokio
- Implement PostgreSQL database schema and connections
- Create REST API endpoints
- Implement password protection system

### Phase 2: Frontend Development
- Create simple, fast time selection interface
- Develop clear availability visualization
- Ensure mobile-friendly design
- Optimize for speed and clarity

### Phase 3: Discord Bot
- Implement bot commands
- Connect to API endpoints
- Develop Discord user tracking
- Create group management functionality
- Implement group matching functionality
- Develop confirmation system using reactions
- Format data for Discord display
- Test and refine

### Phase 4: Testing and Refinement
- Usability testing focusing on speed and clarity
- Performance optimization
- Bug fixes
- Documentation

## Conclusion

This scheduling service focuses on simplicity, speed, and clarity. The Discord bot integration provides essential group management features, allowing users to create groups of Discord users and find optimal meeting times where members from different groups can attend. The confirmation system ensures that suggested times work for the required number of participants, making it easy to coordinate meetings across multiple groups.
