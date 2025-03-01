# TimeSync Architecture

This document describes the architecture of the TimeSync application, detailing the system components, data flow, and design decisions.

## System Overview

TimeSync is a scheduling coordination service built with Rust. It follows a clean architecture approach, separating concerns into distinct layers:

1. **Domain Layer** (core crate)
   - Business entities and rules
   - Domain models
   - Value objects
   - Domain services
   - Error types

2. **Application Layer** (api crate)
   - Use cases and application services
   - DTOs (Data Transfer Objects)
   - Request/response models
   - Coordination of domain layer objects

3. **Interface Layer** (api crate)
   - API endpoints and controllers
   - Request validation
   - Response formatting
   - Authentication and middleware

4. **Infrastructure Layer** (db crate)
   - Database access
   - External service integrations
   - Caching mechanisms
   - Storage implementations

## Component Structure

### Core Crate

The `core` crate is the heart of the system, containing:

- **Models**: Domain entities like `Schedule`, `Availability`, and `TimeSlot`
- **Errors**: Domain-specific error types and handling
- **Services**: Core business logic for scheduling and availability matching
- **DTOs**: Data transfer objects for communication between layers

This crate has no dependencies on external frameworks or libraries, ensuring the business logic remains pure and easily testable.

### API Crate

The `api` crate implements the web API using Axum, and contains:

- **Handlers**: Request processing logic for each endpoint
- **Routes**: URL mapping and endpoint definition
- **Middleware**: Cross-cutting concerns like authentication and error handling
- **Configuration**: Environment and application settings

### Database Crate

The `db` crate manages data persistence using SQLx and PostgreSQL:

- **Repositories**: Data access patterns for each domain entity
- **Migrations**: Database schema evolution
- **Connection**: Database connection pooling and management
- **Mocks**: Testing utilities for database-dependent code

### Discord Bot Crate

The `discord-bot` crate provides Discord integration:

- **Commands**: Discord slash command implementations
- **Events**: Event handlers for Discord interactions
- **Services**: Integration services between Discord and core business logic

## Data Flow

1. **Request Handling**:
   - HTTP request reaches the API server
   - Router directs the request to the appropriate handler
   - Middleware processes authentication and validation
   - Handler extracts and validates request parameters

2. **Business Logic**:
   - Handler delegates to repository for data access
   - Core business logic is applied to data
   - Domain rules and validations are enforced
   - Results are generated based on business logic

3. **Response Generation**:
   - Handler formats domain objects into response DTOs
   - Response is serialized to JSON
   - Middleware applies common response transformations
   - HTTP response is returned to the client

## Key Design Decisions

### Rust and Asynchronous Programming

TimeSync leverages Rust's strong type system and ownership model to build a robust, efficient application. The system uses Tokio for asynchronous runtime, enabling high concurrency with minimal resource usage.

### RESTful API Design

The API follows RESTful principles with:
- Resource-based URL structure
- Appropriate HTTP methods for operations
- Consistent response formats
- Proper status code usage

### Error Handling

A comprehensive error handling strategy:
- Domain-specific error types
- Conversion to appropriate HTTP status codes
- Descriptive error messages
- Consistency across the application

### Security

Security measures include:
- Password hashing using Argon2
- JWT-based authentication for protected endpoints
- Input validation to prevent injection attacks
- HTTPS support for production deployments

### Testing Strategy

The testing approach includes:
- Unit tests for core business logic
- Integration tests for API endpoints
- Mock database for testing repositories
- Test fixtures and factories

## Deployment Architecture

TimeSync can be deployed in various configurations:

### Single-Server Deployment

```
┌─────────────────────────────────┐
│            HTTP/HTTPS           │
└───────────────┬─────────────────┘
                │
┌───────────────▼─────────────────┐
│        Reverse Proxy            │
│      (Nginx, Caddy, etc.)       │
└───────────────┬─────────────────┘
                │
┌───────────────▼─────────────────┐
│        TimeSync API Server      │
└───────────────┬─────────────────┘
                │
┌───────────────▼─────────────────┐
│          PostgreSQL DB          │
└─────────────────────────────────┘
```

### Containerized Deployment

```
┌─────────────────────────────────┐
│            HTTP/HTTPS           │
└───────────────┬─────────────────┘
                │
┌───────────────▼─────────────────┐
│           Load Balancer         │
└───┬───────────────────────┬─────┘
    │                       │
┌───▼───────┐         ┌─────▼─────┐
│ TimeSync  │         │ TimeSync  │
│ Container │         │ Container │
└───┬───────┘         └─────┬─────┘
    │                       │
┌───▼───────────────────────▼─────┐
│       PostgreSQL Container      │
└─────────────────────────────────┘
```

## Performance Considerations

- **Database Optimization**: Proper indexing and query optimization
- **Connection Pooling**: Efficient database connection management
- **Caching**: Strategic caching of frequently accessed data
- **Asynchronous Processing**: Non-blocking I/O for API requests

## Future Enhancements

- **Scalability**: Horizontal scaling with stateless API servers
- **Caching Layer**: Redis integration for improved performance
- **Analytics**: Usage tracking and optimization insights
- **Advanced Matching**: Enhanced availability matching algorithms
- **Mobile Apps**: Native mobile clients for improved user experience