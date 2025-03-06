# IndieFuture Agent CLI - Developer Guide

## Build Commands
- Build: `cargo build --release`
- Run: `cargo run --release`
- Check: `cargo check`
- Format code: `cargo fmt`
- Lint: `cargo clippy`
- Test: `cargo test` or `cargo test <test_name>`

## Code Style Guidelines
- **Imports**: Group by source (std lib first, external crates, internal modules)
- **Formatting**: 4-space indentation, run `cargo fmt` before commits
- **Types**: Use strong typing, define domain-specific types, use `thiserror` for errors
- **Naming**: snake_case for functions/variables, PascalCase for types/enums
- **Error Handling**: Use `AgentResult<T>` and `?` operator, provide context in errors
- **Async Code**: Use `tokio` and `async_trait`, mark I/O functions as async

## Architecture Notes
- AI clients implement the `AiClient` trait (claude.rs, openai.rs)
- Task execution flow: task → subtasks → execution
- Memory system for context tracking and retrieval
- Codebase parsing for project-specific operations