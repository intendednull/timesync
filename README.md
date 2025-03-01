# TimeSync

A lightweight, privacy-focused scheduling coordination service built with Rust.

## Overview

TimeSync helps groups coordinate meeting times without requiring user accounts or collecting unnecessary personal data. Users can create schedules, share them with a unique ID, and visualize where everyone's availability overlaps.

## Features

- **Anonymous Scheduling**: No user accounts required
- **Password Protection**: Optional password protection for schedules
- **Time Slot Management**: Easy-to-use interface for indicating availability
- **Optimal Meeting Time**: Automatic suggestion of optimal meeting times
- **Discord Integration**: Optional Discord bot for community coordination

## Project Structure

The project is organized as a Rust workspace with multiple crates:

- `api`: Axum web server providing RESTful API endpoints
- `core`: Shared domain models and business logic
- `db`: Database interactions with PostgreSQL using SQLx
- `discord-bot`: Discord integration

## Getting Started

### Prerequisites

- Rust 1.70+
- PostgreSQL 14+
- Docker (optional, for containerized development)

### Installation

1. Clone the repository:
   ```
   git clone https://github.com/yourusername/timesync.git
   cd timesync
   ```

2. Set up the database:
   ```
   cp .env.example .env
   # Edit .env with your database credentials
   ```

3. Build the project:
   ```
   cargo build
   ```

4. Run the application:
   ```
   cargo run
   ```

### Development

- Build: `cargo build`
- Run: `cargo run`
- Test: `cargo test`
- Lint: `cargo clippy`
- Format: `cargo fmt`

## API Documentation

The TimeSync API provides endpoints for:

- Schedule creation and management
- Availability registration
- Optimal meeting time calculation
- Discord integration

For detailed API documentation, see [API.md](docs/API.md) or run the server and visit `/docs` for the OpenAPI specification.

## Architecture

TimeSync follows a clean architecture approach:

- **Domain Layer**: Core business logic and models
- **Application Layer**: Use cases and service coordination
- **Interface Layer**: API endpoints and serialization
- **Infrastructure Layer**: Database access and external services

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License - see the LICENSE file for details.