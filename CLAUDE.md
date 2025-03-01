# TimeSync Development Guide

## Build & Test Commands
- Build: `cargo build`
- Run: `cargo run`
- Test all: `cargo test`
- Test single: `cargo test test_name`
- Check: `cargo check`
- Lint: `cargo clippy`
- Format: `cargo fmt`

## Code Style Guidelines
- **Naming**: Use snake_case for variables/functions, PascalCase for types/structs
- **Imports**: Group imports by standard lib, external crates, then internal modules
- **Error Handling**: Use eyre for error handling with context, prefer ? operator
- **Types**: Use strong typing, avoid `impl Trait` for public APIs
- **Documentation**: Document public APIs with /// comments, include examples
- **Formatting**: Follow rustfmt conventions, 4-space indentation
- **Architecture**: RESTful API design with Axum, Tokio for async runtime
- **Database**: Use SQLx for type-safe SQL queries with PostgreSQL
- **Testing**: Write unit tests for core business logic, integration tests for API
- **Performance**: Minimize allocations in hot paths, use async where appropriate