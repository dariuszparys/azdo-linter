# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.0] - 2025-01-12

### Changed

- **Breaking:** Replaced Azure CLI dependency with direct REST API calls
- Authentication now uses Personal Access Token (PAT) instead of Azure CLI session
- New `--pat` / `-t` argument for providing PAT (or use `AZDO_PAT` environment variable)
- Improved error messages with specific guidance for authentication and permission errors

### Added

- `reqwest` HTTP client for Azure DevOps REST API communication
- Support for `AZDO_PAT` environment variable as alternative to `--pat` flag

### Removed

- Azure CLI dependency - the tool no longer requires `az` to be installed

## [0.2.0] - 2025-01-10

### Added

- Pipeline definition variable validation via `--pipeline-id` or `--pipeline-name`
- Variable resolution from three sources: inline YAML, pipeline definition, and variable groups
- Support for variables set in Azure DevOps UI (not just YAML-defined)

## [0.1.0] - 2025-01-08

### Added

- Initial release
- Parse Azure DevOps pipeline YAML files
- Extract variable group references and variable usages
- Validate variable groups exist in Azure DevOps
- Validate referenced variables exist in variable groups
- Support for template files with automatic detection
- Handle variables at top-level, stage, and job scopes
- Support for template conditionals (`${{ if ... }}`) and map-syntax variables
- Filter system variables, PowerShell expressions, and runtime outputs
- Clear error messages with suggestions
- Exit codes for CI/CD integration (0=success, 1=validation failure, 2=error)

[Unreleased]: https://github.com/dariuszparys/azdo-linter/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/dariuszparys/azdo-linter/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/dariuszparys/azdo-linter/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/dariuszparys/azdo-linter/releases/tag/v0.1.0
