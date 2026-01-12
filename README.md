# Azure DevOps Pipeline Validator

[![Crates.io](https://img.shields.io/crates/v/azdolint.svg)](https://crates.io/crates/azdolint)

A command-line tool that validates Azure DevOps pipeline YAML files by checking that all referenced variable groups and variables actually exist in Azure DevOps.

## Features

- Parses Azure DevOps pipeline YAML files
- Extracts variable group references and variable usages
- Validates that variable groups exist in Azure DevOps
- Validates that referenced variables exist in the variable groups
- Validates pipeline definition variables (set via Azure DevOps UI) in addition to YAML-defined variables
- Resolves variable references from three sources (inline YAML, pipeline definition, variable groups)
- Supports template files with automatic detection and validation in parent context
- Handles variables at top-level, stage, and job scopes
- Supports template conditionals (`${{ if ... }}`) and map-syntax variables
- Provides clear, actionable error messages with suggestions
- Returns appropriate exit codes for CI/CD integration

## Prerequisites

### Azure CLI

This tool requires the Azure CLI with the Azure DevOps extension installed and configured.

1. **Install Azure CLI**: Follow the [official installation guide](https://docs.microsoft.com/en-us/cli/azure/install-azure-cli)

2. **Install Azure DevOps Extension**:
   ```bash
   az extension add --name azure-devops
   ```

3. **Login to Azure**:
   ```bash
   az login
   ```

4. **Configure Default Organization** (optional):
   ```bash
   az devops configure --defaults organization=https://dev.azure.com/YOUR_ORG
   ```

## Installation

### From crates.io (Recommended)

```bash
cargo install azdolint
```

### From Source

```bash
# Clone the repository
git clone https://github.com/dariuszparys/azdo-linter.git
cd azdo-linter

# Build the project
cargo build --release

# The binary will be available at target/release/azdolint
```

### Using Cargo (local)

```bash
cargo install --path .
```

## Usage

```bash
azdolint --pipeline-file <PATH> --organization <ORG> --project <PROJECT> [OPTIONS]
```

### Arguments

| Argument | Short | Description |
|----------|-------|-------------|
| `--pipeline-file` | `-p` | Path to the Azure DevOps pipeline YAML file to validate |
| `--organization` | `-o` | Azure DevOps organization name or URL |
| `--project` | `-j` | Azure DevOps project name |
| `--pipeline-name` | `-n` | Optional: Pipeline name in Azure DevOps (enables pipeline definition variable validation) |
| `--pipeline-id` | `-i` | Optional: Pipeline ID in Azure DevOps (more reliable than name, find it in URL as pipelineId=XXX) |
| `--verbose` | `-v` | Enable verbose output for debugging |

### Examples

**Basic validation:**
```bash
azdolint --pipeline-file azure-pipelines.yml --organization myorg --project myproject
```

**With full organization URL:**
```bash
azdolint -p azure-pipelines.yml -o https://dev.azure.com/myorg -j myproject
```

**Verbose output:**
```bash
azdolint -p azure-pipelines.yml -o myorg -j myproject --verbose
```

**With pipeline definition variable validation:**
```bash
azdolint -p azure-pipelines.yml -o myorg -j myproject --pipeline-id 42
```

Or using pipeline name:
```bash
azdolint -p azure-pipelines.yml -o myorg -j myproject --pipeline-name "My Pipeline"
```

## Exit Codes

The validator uses the following exit codes for CI/CD integration:

| Exit Code | Meaning |
|-----------|---------|
| `0` | Success - All variable groups and variables exist |
| `1` | Validation failure - Some variable groups or variables were not found |
| `2` | Error - Could not complete validation (e.g., Azure CLI not available, file not found) |

### CI/CD Integration Example

```yaml
# Azure DevOps Pipeline
steps:
  - script: |
      azdolint --pipeline-file azure-pipelines.yml \
        --organization $(System.CollectionUri) \
        --project $(System.TeamProject)
    displayName: 'Validate Pipeline Variables'
```

## Sample Output

### Successful Validation
```
Azure DevOps Pipeline Validator
================================

Variable Groups
---------------
  [PASS] Variable group 'ProductionSecrets' exists
  [PASS] Variable group 'DatabaseConfig' exists

Variable References
-------------------
  [PASS] Variable 'ConnectionString' found in group 'DatabaseConfig'
  [PASS] Variable 'ApiKey' found in group 'ProductionSecrets'

================================
RESULT: PASSED
All 4 check(s) passed successfully.
================================
```

### Failed Validation
```
Azure DevOps Pipeline Validator
================================

Variable Groups
---------------
  [PASS] Variable group 'ProductionSecrets' exists
  [FAIL] Variable group 'MissingGroup' not found
         Suggestion: Create the variable group in Azure DevOps at:
         https://dev.azure.com/myorg/myproject/_library?itemType=VariableGroups

Variable References
-------------------
  [PASS] Variable 'ApiKey' found in group 'ProductionSecrets'
  [FAIL] Variable 'UndefinedVar' not found in any referenced group
         Suggestion: Add this variable to one of the referenced variable groups,
         or verify the variable name is spelled correctly.

================================
RESULT: FAILED
2 of 4 check(s) failed.
================================
```

## Variable Resolution

When validating variable references, the tool checks three sources in priority order:

1. **Inline Variables** - Variables defined directly in the pipeline YAML file
2. **Pipeline Definition Variables** - Variables set on the pipeline definition in Azure DevOps (requires `--pipeline-id` or `--pipeline-name`)
3. **Variable Groups** - Variables defined in Azure DevOps library variable groups

This means if a variable is defined in multiple places, the tool will find it and consider it valid. To enable pipeline definition variable validation, provide either `--pipeline-id` (recommended) or `--pipeline-name`.

**Note:** Due to an Azure CLI bug, `--pipeline-id` is more reliable than `--pipeline-name`. You can find the pipeline ID in the Azure DevOps URL as `pipelineId=XXX`.

## Supported Pipeline Syntax

The validator supports the following variable definition formats in Azure DevOps YAML:

### Variable Groups
```yaml
variables:
  - group: 'MyVariableGroup'
```

### Inline Variables
```yaml
# List format
variables:
  - name: BuildConfiguration
    value: 'Release'

# Map format
variables:
  BuildConfiguration: 'Release'
```

### Variable References
The validator detects variable references using the `$(variableName)` syntax anywhere in the pipeline YAML.

### Template Conditionals
```yaml
variables:
  - ${{ if eq(parameters.environment, 'prod') }}:
    - group: 'ProductionSecrets'
  - ${{ else }}:
    - group: 'DevelopmentSecrets'
```

### Stage and Job Scoped Variables
Variables defined at stage or job level are properly scoped and validated:
```yaml
stages:
  - stage: Build
    variables:
      - group: 'BuildSecrets'
    jobs:
      - job: BuildJob
        variables:
          - name: JobVar
            value: 'value'
```

### Template Files
Template files are automatically detected (files with `parameters:` but no `trigger:`). When run against a template directly, the linter shows a warning and skips validation. Templates are validated in the context of the parent pipeline that includes them.

## License

MIT License

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
