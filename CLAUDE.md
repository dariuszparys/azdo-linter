# Project: azdo-linter

CLI tool that validates Azure DevOps pipeline YAML files by checking that referenced variable groups and variables exist in Azure DevOps.

## Stack
- Rust 2021 edition
- clap 4.0 (CLI parsing)
- serde_yaml 0.9 (YAML parsing)
- anyhow 1.0 (error handling)
- Azure CLI required at runtime (az devops extension)

## Commands
- `cargo build` — build the project
- `cargo test` — run unit and integration tests
- `cargo clippy -- -D warnings` — lint (must pass with no warnings)
- `cargo run -- -p <file> -o <org> -j <project>` — run validator

## Structure
```
src/
  main.rs      — CLI entry point, orchestrates validation workflow
  lib.rs       — module exports
  parser.rs    — YAML parsing, extracts variable groups and references
  azure.rs     — Azure DevOps API client (calls az CLI)
  validator.rs — validation logic for groups and variables
  error.rs     — output formatting and error types
tests/
  integration_tests.rs — parser integration tests
  fixtures/            — sample pipeline YAML files
```

## Rules
- All clippy warnings must be fixed before committing
- Exit codes: 0=success, 1=validation failure, 2=error
- Variable references use `$(varName)` syntax
- Filter out system variables, PowerShell expressions, and runtime outputs when parsing
- **Customer pipelines**: Pipeline files passed for debugging are customer data. Never copy them directly into test fixtures. Create neutral, anonymized test cases that reproduce the issue without customer-specific content.

## Domain
- **Variable groups**: Named collections of variables in Azure DevOps library
- **Inline variables**: Variables defined directly in pipeline YAML
- **Variable references**: `$(varName)` syntax used throughout pipeline
- Parser must handle variables at top-level, stage, and job scopes
