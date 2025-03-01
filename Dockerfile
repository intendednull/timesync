FROM rust:1.75-slim-bookworm as builder

WORKDIR /usr/src/app
COPY . .

# Install dependencies for building
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Install cargo-watch for development
RUN cargo install cargo-watch

# Build the release binaries
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy binaries from builder stage
COPY --from=builder /usr/src/app/target/release/timesync /app/timesync
COPY --from=builder /usr/src/app/target/release/discord-bot /app/discord-bot
COPY --from=builder /usr/src/app/target/release/db-migrate /app/db-migrate

# Copy static files
COPY src/*.html src/*.js src/*.css /app/static/

# Create a non-root user to run the application
RUN useradd -m appuser
RUN chown -R appuser:appuser /app
USER appuser

# Set environment variable for static files
ENV STATIC_FILES_DIR=/app/static

# Default command runs the API server
CMD ["/app/timesync"]