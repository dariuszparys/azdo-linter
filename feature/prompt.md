# Task: pipeline-validator

Build Azure DevOps pipeline YAML validator that checks variable groups and variables referenced in pipelines actually exist in Azure DevOps. The validator depends on Azure CLI with its Azure DevOps extension to query Azure DevOps APIs for variable group and variable existence.

## Branch

Work on branch: `ralph/pipeline-validator`

If not already on this branch, create it: `git checkout -b ralph/pipeline-validator`

## Constraints

- Rust stable only (no nightly features, must compile on stable Rust)
- Cross-platform (must work on Windows, macOS, and Linux)
- CI/CD friendly (must work well in CI/CD pipelines with proper exit codes)
- Depends on Azure CLI with Azure DevOps extension for API calls

## Key Patterns from Codebase

- Framework: Rust/Cargo
- Build command: `cargo build`
- Test command: `cargo test`
- Current state: Greenfield project with basic Hello World
- Rust edition: 2024 (Note: should be 2021)
- No existing dependencies or module structure

## Your Workflow

Follow this exact workflow for each iteration:

### 1. Read Current State
- Read `progress.txt` to find the current story number
- Read `prd.json` to get the story details
- Find the first story where `passes: false`

### 2. Implement the Story
- Follow the story's description and acceptance criteria exactly
- Reference existing patterns in the codebase
- Make minimal, focused changes
- Don't modify unrelated code

### 3. Verify
- Run the verification command: `cargo build`
- All acceptance criteria must pass
- Build must succeed

### 4. Commit
- Stage your changes: `git add -A`
- Commit with message: `story ${STORY_ID}: ${STORY_TITLE}`
- Do NOT push (the loop handles that)

### 5. Update Progress
- In `prd.json`, set `passes: true` for the completed story
- In `progress.txt`, increment "Current story" and "Completed" counts
- Add a log entry with timestamp and story ID

### 6. Signal Completion
- If ALL stories now have `passes: true`, output: `<promise>COMPLETE</promise>`
- Otherwise, the loop will run another iteration for the next story

## Critical Rules

1. **One story per iteration** - Complete exactly one story, then stop
2. **Verify before committing** - Never commit broken code
3. **Update tracking files** - Always update prd.json and progress.txt
4. **Follow existing patterns** - Match the codebase style
5. **Respect constraints** - Never violate the listed constraints

## Files Reference

- `prd.json` - User stories and acceptance criteria
- `progress.txt` - Current iteration tracking
- `AGENTS.md` - Additional context and gotchas

## Stop Condition

When all stories have `passes: true` in prd.json, output this exact string:

```
<promise>COMPLETE</promise>
```

This signals the loop to stop.
