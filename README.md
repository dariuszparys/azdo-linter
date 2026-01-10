# Azure DevOps Pipeline Validator

A command-line tool that validates Azure DevOps pipeline YAML files by checking that all referenced variable groups and variables actually exist in Azure DevOps.

## Features

- Parses Azure DevOps pipeline YAML files
- Extracts variable group references and variable usages
- Validates that variable groups exist in Azure DevOps
- Validates that referenced variables exist in the variable groups
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

### From Source

```bash
# Clone the repository
git clone https://github.com/your-username/azdo-linter.git
cd azdo-linter

# Build the project
cargo build --release

# The binary will be available at target/release/azdo-linter
```

### Using Cargo

```bash
cargo install --path .
```

## Usage

```bash
azdo-linter --pipeline-file <PATH> --organization <ORG> --project <PROJECT> [OPTIONS]
```

### Arguments

| Argument | Short | Description |
|----------|-------|-------------|
| `--pipeline-file` | `-p` | Path to the Azure DevOps pipeline YAML file to validate |
| `--organization` | `-o` | Azure DevOps organization name or URL |
| `--project` | `-j` | Azure DevOps project name |
| `--verbose` | `-v` | Enable verbose output for debugging |

### Examples

**Basic validation:**
```bash
azdo-linter --pipeline-file azure-pipelines.yml --organization myorg --project myproject
```

**With full organization URL:**
```bash
azdo-linter -p azure-pipelines.yml -o https://dev.azure.com/myorg -j myproject
```

**Verbose output:**
```bash
azdo-linter -p azure-pipelines.yml -o myorg -j myproject --verbose
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
      azdo-linter --pipeline-file azure-pipelines.yml \
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

## Supported Pipeline Syntax

The validator supports the following variable definition formats in Azure DevOps YAML:

### Variable Groups
```yaml
variables:
  - group: 'MyVariableGroup'
```

### Inline Variables
```yaml
variables:
  - name: BuildConfiguration
    value: 'Release'
```

### Variable References
The validator detects variable references using the `$(variableName)` syntax anywhere in the pipeline YAML.

## License

MIT License

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
