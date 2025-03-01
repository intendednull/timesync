# TimeSync Development Guide

This guide provides instructions for setting up and developing the TimeSync project.

## Prerequisites

Before you begin, ensure you have the following installed:

- **Rust** (1.70 or later): [Install Rust](https://www.rust-lang.org/tools/install)
- **PostgreSQL** (14 or later): [Install PostgreSQL](https://www.postgresql.org/download/)
- **Docker** (optional): [Install Docker](https://docs.docker.com/get-docker/)
- **Git**: [Install Git](https://git-scm.com/downloads)

## Initial Setup

### 1. Clone the Repository

```bash
git clone https://github.com/yourusername/timesync.git
cd timesync
```

### 2. Environment Configuration

Copy the example environment file and configure it for your local setup:

```bash
cp .env.example .env
```

Edit the `.env` file with your database credentials and other settings:

```
# Database configuration
DATABASE_URL=postgres://username:password@localhost:5432/timesync
DATABASE_POOL_SIZE=5

# API configuration
API_HOST=127.0.0.1
API_PORT=8080

# Discord configuration (optional)
DISCORD_BOT_TOKEN=your_discord_bot_token
DISCORD_CLIENT_ID=your_discord_client_id
```

### 3. Database Setup

Create a PostgreSQL database for the project:

```bash
psql -U postgres -c "CREATE DATABASE timesync;"
```

### 4. Run Database Migrations

Apply the database migrations:

```bash
cargo run --bin migrate
```

## Development Workflow

### Running the Application

Start the API server:

```bash
cargo run
```

For watch mode with auto-reloading (requires `cargo-watch`):

```bash
cargo watch -x run
```

### Testing

Run all tests:

```bash
cargo test
```

Run a specific test:

```bash
cargo test test_name
```

Run tests with output:

```bash
cargo test -- --nocapture
```

### Code Quality

Run the linter:

```bash
cargo clippy
```

Fix formatting issues:

```bash
cargo fmt
```

Check for compilation errors without building:

```bash
cargo check
```

## Project Structure

The TimeSync project is organized as a workspace with multiple crates:

```
timesync/
├── .env.example        # Example environment variables
├── Cargo.toml          # Workspace definition
├── README.md           # Project overview
├── docs/               # Documentation
│   ├── API.md          # API documentation
│   ├── ARCHITECTURE.md # Architecture documentation
│   └── DEVELOPMENT.md  # Development guide
├── crates/
│   ├── api/            # RESTful API implementation
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   └── tests/
│   ├── core/           # Domain models and business logic
│   │   ├── Cargo.toml
│   │   └── src/
│   ├── db/             # Database access
│   │   ├── Cargo.toml
│   │   ├── migrations/
│   │   └── src/
│   └── discord-bot/    # Discord integration
│       ├── Cargo.toml
│       └── src/
└── scripts/            # Utility scripts
```

## Adding New Features

When adding new features, follow these steps:

1. **Core Models**: Add domain entities to the `core` crate
2. **Database Layer**: Update repositories in the `db` crate
3. **API Endpoints**: Implement handlers and routes in the `api` crate
4. **Tests**: Write tests for each component
5. **Documentation**: Update API documentation

## Working with Database

### Creating a Migration

Create a new database migration:

```bash
cd crates/db
sqlx migrate add migration_name
```

### Running Migrations

Apply pending migrations:

```bash
cargo run --bin migrate
```

Revert the last migration:

```bash
cargo run --bin migrate -- revert
```

## Docker Development Environment

A Docker Compose file is provided for development:

```bash
docker-compose up -d
```

This starts:
- PostgreSQL database
- PgAdmin web interface (optional)

## Testing with Dummy Data

Load test data into the database:

```bash
cargo run --bin seed
```

## API Documentation

OpenAPI documentation is available at `/docs` when the server is running.

For a static version, see `docs/API.md`.

## Troubleshooting

### Database Connection Issues

If you encounter database connection issues:

1. Verify PostgreSQL is running:
   ```bash
   pg_isready
   ```

2. Check your connection string in `.env`

3. Ensure your database user has proper permissions:
   ```bash
   psql -U postgres -c "ALTER USER your_user WITH SUPERUSER;"
   ```

### Compilation Errors

For Rust compilation errors:

1. Update Rust toolchain:
   ```bash
   rustup update
   ```

2. Clean and rebuild:
   ```bash
   cargo clean && cargo build
   ```

### Getting Help

If you encounter issues:

1. Check the existing issues on GitHub
2. Join our community Discord channel
3. Open a new issue with detailed error information

## Release Process

1. Update version in `Cargo.toml` files
2. Update changelog
3. Run all tests and linting
4. Tag the release in Git
5. Build release artifacts
6. Deploy to staging for verification
7. Deploy to production