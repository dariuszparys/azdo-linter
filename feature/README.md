# pipeline-validator

Build Azure DevOps pipeline YAML validator that checks variable groups and variables referenced in pipelines actually exist in Azure DevOps.

## Quick Start

```bash
# Make the loop script executable
chmod +x ralph.sh

# Run the autonomous loop
./ralph.sh

# Or run with custom max iterations
MAX_ITERATIONS=20 ./ralph.sh
```

## Files

| File | Purpose |
|------|---------|
| `ralph.sh` | Loop runner script - executes Claude Code iterations |
| `prompt.md` | AI prompt context - instructions for each iteration |
| `prd.json` | User stories - atomic tasks in dependency order |
| `progress.txt` | Progress tracking - current story and completed log |
| `AGENTS.md` | Agent instructions - gotchas, patterns, constraints |
| `README.md` | This file - usage guide |

## User Stories

**Total: 20 stories**

1. **US-001:** Add core dependencies to Cargo.toml
2. **US-002:** Create project module structure
3. **US-003:** Define core data structures for pipeline YAML
4. **US-004:** Implement YAML file parser function
5. **US-005:** Create function to extract variable group names from pipeline
6. **US-006:** Create function to extract variable references from pipeline YAML
7. **US-007:** Create Azure CLI wrapper struct
8. **US-008:** Implement method to fetch variable group from Azure DevOps
9. **US-009:** Implement method to get variables from a variable group
10. **US-010:** Create validation logic for variable groups
11. **US-011:** Create validation logic for variables in groups
12. **US-012:** Define CLI argument structure with clap
13. **US-013:** Implement main CLI orchestration logic
14. **US-014:** Add error formatting and user-friendly output
15. **US-015:** Create integration test for YAML parsing
16. **US-016:** Create unit tests for Azure CLI wrapper
17. **US-017:** Create unit tests for validation logic
18. **US-018:** Add README with usage instructions
19. **US-019:** Add CI/CD configuration for cross-platform testing
20. **US-020:** Add Cargo.toml metadata for publishing

## Monitoring

```bash
# Watch the log file
tail -f ralph.log

# Check current progress
cat progress.txt

# See remaining stories
jq '[.userStories[] | select(.passes == false)] | length' prd.json
```

## Troubleshooting

### Loop stops unexpectedly
- Check `ralph.log` for errors
- Verify `progress.txt` has correct current story number
- Ensure `prd.json` acceptance criteria are achievable

### Story keeps failing
- Review the acceptance criteria in `prd.json`
- Check if the story is too large (should be 5-30 min of work)
- Look for missing dependencies or constraints

### Build failures
- Check that the build command is correct
- Verify all dependencies are installed
- Review recent changes in git log

## Manual Intervention

If you need to fix something manually:

1. Stop the loop (Ctrl+C)
2. Make your fixes
3. Update `progress.txt` if needed
4. Restart: `./ralph.sh`

## Branch

All work is on branch: `ralph/pipeline-validator`

```bash
# Switch to this branch
git checkout ralph/pipeline-validator

# See all commits
git log --oneline
```
