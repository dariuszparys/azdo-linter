# Task: pipeline-validator

Build Azure DevOps pipeline YAML validator that checks variable groups and variables referenced in pipelines actually exist in Azure DevOps. The validator depends on Azure CLI with its Azure DevOps extension to query Azure DevOps APIs for variable group and variable existence.

## Your Task

1. **Read context files**:
   - `prd.json` - User stories for this task
   - `progress.txt` - Check **Codebase Patterns** section FIRST
   - `AGENTS.md` - Additional context and constraints

2. **Check branch**: Ensure you're on the correct branch
   ```bash
   git checkout ralph/pipeline-validator 2>/dev/null || git checkout -b ralph/pipeline-validator
   ```

3. **Pick next story**: Select highest priority story where `passes: false`

4. **Implement ONE story**:
   - Follow patterns from progress.txt Codebase Patterns section
   - Reference existing code as templates
   - Keep changes minimal and focused

5. **Verify**:
   ```bash
   cargo build && cargo test
   ```

6. **Commit** (if build passes):
   ```bash
   git add -A
   git commit -m "story [STORY-ID]: [Title]"
   ```

7. **Update prd.json**: Set `passes: true` for completed story

8. **Append to progress.txt**:
   ```markdown
   ## [Date] - [Story ID]
   - What was implemented
   - Files changed: [list]
   - **Learnings:**
     - Any patterns discovered
     - Gotchas encountered
   ---
   ```

   If you discovered important patterns or gotchas, also add them to the **Codebase Patterns** section at the top of progress.txt so future iterations can benefit.

## Constraints

- Rust stable only (no nightly features, must compile on stable Rust)
- Cross-platform (must work on Windows, macOS, and Linux)
- CI/CD friendly (must work well in CI/CD pipelines with proper exit codes)
- Depends on Azure CLI with Azure DevOps extension for API calls

## Key Patterns from Codebase

- Framework: Rust/Cargo
- Build command: `cargo build && cargo test`
- Module structure: `src/lib.rs` with `parser`, `azure`, `validator`, `error` modules
- Uses serde with derive for YAML/JSON parsing
- Uses clap with derive for CLI argument parsing
- Uses anyhow for error handling
- Uses regex for variable extraction from pipeline YAML

## Critical Rules

1. **One story per iteration** - Complete exactly one story, then stop
2. **Read progress.txt first** - Check Codebase Patterns for learnings from previous iterations
3. **Capture learnings** - Always document patterns and gotchas discovered
4. **Verify before committing** - Never commit broken code
5. **Update tracking files** - Always update prd.json and progress.txt
6. **Follow existing patterns** - Match the codebase style
7. **Respect constraints** - Never violate the listed constraints

## Files Reference

- `prd.json` - User stories and acceptance criteria
- `progress.txt` - Current iteration tracking and accumulated learnings
- `AGENTS.md` - Additional context and gotchas

## Stop Condition

If **ALL stories** in prd.json have `passes: true`, output:

```
<promise>COMPLETE</promise>
```

Otherwise, end your response normally after completing and committing ONE story.
