# Docker Setup for TimeSync

This document explains how to use Docker with the TimeSync application.

## Prerequisites

- [Docker](https://docs.docker.com/get-docker/)
- [Docker Compose](https://docs.docker.com/compose/install/)

## Quick Start

1. Clone the repository:
   ```bash
   git clone https://github.com/yourusername/timesync.git
   cd timesync
   ```

2. Set up your environment variables:
   ```bash
   cp .env.example .env
   # Edit .env with your desired configuration
   ```

3. Start the services:
   ```bash
   docker-compose up -d
   ```

4. Access the web application at: [http://localhost:3000](http://localhost:3000)

## Available Services

- **API Server**: The main TimeSync API service
- **PostgreSQL Database**: Persistent storage for schedules and availability
- **Discord Bot**: (Optional) For Discord integration
- **Database Migration**: Runs automatically on startup to set up the database schema

## Development Setup

For development, an override file is provided that enables live code reloading and additional development tools:

```bash
# Start with development configuration
docker-compose -f docker-compose.yml -f docker-compose.override.yml up -d

# Access pgAdmin for database management
# Available at http://localhost:5050
# Login with admin@example.com / admin (or as configured in .env)
```

The development setup includes:
- Live code reloading using cargo-watch
- Volume mounts for the codebase
- pgAdmin for database management
- Cargo registry caching for faster builds

## Running the Discord Bot

The Discord bot requires a valid Discord token. If you wish to run the bot:

1. Add your Discord token to the .env file:
   ```
   DISCORD_TOKEN=your_discord_bot_token
   DISCORD_APPLICATION_ID=your_discord_application_id
   ```

2. Start the services with the discord bot profile:
   ```bash
   docker-compose --profile with-discord up -d
   ```

## Production Deployment

For production deployment:

1. Set secure passwords and settings in .env
2. Build and start the containers:
   ```bash
   docker-compose build
   docker-compose up -d
   ```

3. For production environments, consider:
   - Adding a reverse proxy like Nginx or Traefik
   - Setting up SSL certificates
   - Implementing automatic backups for the database volume

## Container Management

Common commands:

```bash
# View logs
docker-compose logs -f

# View logs for a specific service
docker-compose logs -f api

# Stop all services
docker-compose down

# Rebuild after changes
docker-compose build

# Rebuild and start a specific service
docker-compose up -d --build api

# Remove all data (including database)
docker-compose down -v
```

## Database Management

The PostgreSQL database data is persisted in a Docker volume. To back up your data:

```bash
# Create a backup
docker-compose exec postgres pg_dump -U postgres timesync > backup.sql

# Restore from backup
cat backup.sql | docker-compose exec -T postgres psql -U postgres timesync
```

## Troubleshooting

### Common Issues

1. **Database connection error**:
   - Check that the database service is running: `docker-compose ps`
   - Verify connection settings in .env file

2. **Discord bot not connecting**:
   - Verify your Discord token is correct
   - Check bot permissions and invitation in Discord Developer Portal

3. **Container won't start**:
   - Check logs: `docker-compose logs <service-name>`
   - Verify all required environment variables are set

4. **Application taking too long to start**:
   - Initial build may take time on slower systems
   - Watch progress with: `docker-compose logs -f`

### Viewing Logs

```bash
# View logs for all services
docker-compose logs

# View logs for a specific service
docker-compose logs api

# Follow logs in real-time
docker-compose logs -f
```

## Customization

The Docker setup can be customized by modifying:

- `.env` file for environment variables
- `docker-compose.override.yml` for development-specific changes
- `Dockerfile` and `Dockerfile.discord` for container configuration