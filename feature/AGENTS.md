# pipeline-validator - Agent Instructions

## Task Overview

Build Azure DevOps pipeline YAML validator that checks variable groups and variables referenced in pipelines actually exist in Azure DevOps. The validator depends on Azure CLI with its Azure DevOps extension to query Azure DevOps APIs for variable group and variable existence.

## Prior Work

- Greenfield project (starting from scratch)
- Basic Cargo.toml and main.rs exist with Hello World
- No existing patterns or modules to follow

## Key Files and Paths

- **Source:** src/
- **Config:** Cargo.toml
- **Tests:** tests/ (to be created)
- **Main entry:** src/main.rs
- **Library:** src/lib.rs (to be created)
- **Modules:** src/parser.rs, src/azure.rs, src/validator.rs, src/error.rs (to be created)

## Gotchas and Edge Cases

- **Azure DevOps YAML syntax:** AzDO uses special syntax like template expressions ${{ }}
- **Variable substitution:** Variables use $(variableName) syntax - need to parse and extract these
- **Variable groups:** Referenced in YAML and must be validated against Azure DevOps API
- **Multi-format variables section:** Can contain both inline variables and group references
  - Inline: `variables: { myVar: value }`
  - Group reference: `variables: - group: 'GroupName'`
- **Rust edition:** Cargo.toml has edition = "2024" which should be "2021"

## Scope

- [x] YAML parser for Azure DevOps pipeline files
- [x] CLI interface for running the validator
- [x] Integration with Azure CLI/az devops for checking variable groups
- [x] Validate referenced variables exist in variable groups

## Workflow

1. **Read progress.txt** to find the current story number
2. **Read prd.json** to get story details and acceptance criteria
3. **Implement the story** following existing codebase patterns
4. **Run verification**: `cargo build`
5. **Commit** with message: `story US-XXX: <title>`
6. **Update progress.txt** with the completed story
7. **Update prd.json** setting `passes: true` for the story

## Commit Message Format

```
story US-XXX: <short title>

<brief description of what was done>
```

## Quality Checklist

Before committing each story:

- [ ] Acceptance criteria met
- [ ] Build passes
- [ ] Tests pass (if applicable)
- [ ] No unrelated changes included
- [ ] Code follows existing patterns
- [ ] progress.txt updated
- [ ] prd.json updated

## Constraints

- Rust stable only (no nightly features, must compile on stable Rust)
- Cross-platform (must work on Windows, macOS, and Linux)
- CI/CD friendly (must work well in CI/CD pipelines with proper exit codes)
- Depends on Azure CLI with Azure DevOps extension for API calls
