//! Integration tests for Azure DevOps pipeline YAML parsing

use azdo_linter::parser::{extract_variable_references, parse_pipeline_file, VariableEntry};

/// Test parsing a pipeline file with variable groups only
#[test]
fn test_parse_pipeline_with_variable_groups() {
    let path = "tests/fixtures/pipeline_with_groups.yml";
    let pipeline = parse_pipeline_file(path).expect("Failed to parse pipeline file");

    // Should have 2 variable groups
    let groups = pipeline.get_variable_groups();
    assert_eq!(groups.len(), 2);
    assert!(groups.contains(&"ProductionSecrets".to_string()));
    assert!(groups.contains(&"DatabaseConfig".to_string()));
}

/// Test parsing a pipeline file with inline variables only
#[test]
fn test_parse_pipeline_with_inline_variables() {
    let path = "tests/fixtures/pipeline_with_inline_vars.yml";
    let pipeline = parse_pipeline_file(path).expect("Failed to parse pipeline file");

    // Should have no variable groups (all inline)
    let groups = pipeline.get_variable_groups();
    assert!(groups.is_empty());

    // Should have inline variables in the variables section
    let variables = pipeline.variables.expect("Variables section should exist");
    assert_eq!(variables.len(), 2);

    // Verify the named variables
    let mut found_build_config = false;
    let mut found_dotnet_version = false;

    for entry in &variables {
        if let VariableEntry::Named { name, value } = entry {
            if name == "BuildConfiguration" {
                found_build_config = true;
                assert_eq!(value.as_deref(), Some("Release"));
            }
            if name == "DotNetVersion" {
                found_dotnet_version = true;
                assert_eq!(value.as_deref(), Some("6.0.x"));
            }
        }
    }

    assert!(found_build_config, "BuildConfiguration variable not found");
    assert!(found_dotnet_version, "DotNetVersion variable not found");
}

/// Test parsing a pipeline file with both variable groups and inline variables
#[test]
fn test_parse_pipeline_mixed() {
    let path = "tests/fixtures/pipeline_mixed.yml";
    let pipeline = parse_pipeline_file(path).expect("Failed to parse pipeline file");

    // Should have 2 variable groups
    let groups = pipeline.get_variable_groups();
    assert_eq!(groups.len(), 2);
    assert!(groups.contains(&"CommonSecrets".to_string()));
    assert!(groups.contains(&"DeploymentConfig".to_string()));

    // Should have 4 total entries (2 groups + 2 inline vars)
    let variables = pipeline.variables.expect("Variables section should exist");
    assert_eq!(variables.len(), 4);
}

/// Test extracting variable references from pipeline with variable groups
#[test]
fn test_extract_variable_references_from_groups_pipeline() {
    let path = "tests/fixtures/pipeline_with_groups.yml";
    let var_refs = extract_variable_references(path).expect("Failed to extract variable references");

    // Should find 2 variable references: ConnectionString and ApiKey
    assert_eq!(var_refs.len(), 2);
    assert!(var_refs.contains(&"ConnectionString".to_string()));
    assert!(var_refs.contains(&"ApiKey".to_string()));
}

/// Test extracting variable references from pipeline with inline variables
#[test]
fn test_extract_variable_references_from_inline_pipeline() {
    let path = "tests/fixtures/pipeline_with_inline_vars.yml";
    let var_refs = extract_variable_references(path).expect("Failed to extract variable references");

    // Should find 2 variable references: BuildConfiguration and DotNetVersion
    assert_eq!(var_refs.len(), 2);
    assert!(var_refs.contains(&"BuildConfiguration".to_string()));
    assert!(var_refs.contains(&"DotNetVersion".to_string()));
}

/// Test extracting variable references from mixed pipeline
#[test]
fn test_extract_variable_references_from_mixed_pipeline() {
    let path = "tests/fixtures/pipeline_mixed.yml";
    let var_refs = extract_variable_references(path).expect("Failed to extract variable references");

    // Should find 5 unique variable references
    assert_eq!(var_refs.len(), 5);
    assert!(var_refs.contains(&"Environment".to_string()));
    assert!(var_refs.contains(&"ApiKey".to_string()));
    assert!(var_refs.contains(&"Timeout".to_string()));
    assert!(var_refs.contains(&"DeploymentToken".to_string()));
    assert!(var_refs.contains(&"ConnectionString".to_string()));
}

/// Test that parsing a non-existent file returns an error
#[test]
fn test_parse_nonexistent_file() {
    let result = parse_pipeline_file("tests/fixtures/nonexistent.yml");
    assert!(result.is_err());
}

/// Test that variable group extraction returns unique names
#[test]
fn test_variable_groups_are_unique() {
    // This test verifies the get_variable_groups method handles duplicates
    // Even if the same group is referenced multiple times, it should only appear once
    let path = "tests/fixtures/pipeline_with_groups.yml";
    let pipeline = parse_pipeline_file(path).expect("Failed to parse pipeline file");

    let groups = pipeline.get_variable_groups();
    let unique_count = groups.iter().collect::<std::collections::HashSet<_>>().len();

    assert_eq!(groups.len(), unique_count, "Variable groups should be unique");
}

/// Test parsing a pipeline with stage-level variable groups
#[test]
fn test_parse_pipeline_with_stage_level_groups() {
    let path = "tests/fixtures/pipeline_with_stages.yml";
    let pipeline = parse_pipeline_file(path).expect("Failed to parse pipeline file");

    // Should find 3 variable groups from different levels:
    // - build-secrets (stage level)
    // - job-level-group (job level)
    // - deploy-secrets (stage level)
    let groups = pipeline.get_variable_groups();
    assert_eq!(groups.len(), 3);
    assert!(groups.contains(&"build-secrets".to_string()));
    assert!(groups.contains(&"job-level-group".to_string()));
    assert!(groups.contains(&"deploy-secrets".to_string()));
}

/// Test inline variable extraction from stage-level definitions
#[test]
fn test_inline_variables_from_stages() {
    let path = "tests/fixtures/pipeline_with_stages.yml";
    let pipeline = parse_pipeline_file(path).expect("Failed to parse pipeline file");

    // Should find inline variables from top level and stage level:
    // - platformBuildNumber (top level)
    // - buildConfig (stage level)
    let inline_vars = pipeline.get_inline_variable_names();
    assert_eq!(inline_vars.len(), 2);
    assert!(inline_vars.contains(&"platformBuildNumber".to_string()));
    assert!(inline_vars.contains(&"buildConfig".to_string()));
}

/// Test that PowerShell expressions are filtered from variable references
#[test]
fn test_filter_powershell_expressions() {
    let path = "tests/fixtures/pipeline_with_filtering.yml";
    let var_refs = extract_variable_references(path).expect("Failed to extract variable references");

    // Should NOT contain PowerShell expressions
    assert!(!var_refs.iter().any(|v| v.starts_with('$')));
    assert!(!var_refs.contains(&"$outputs.registryName.value".to_string()));
    assert!(!var_refs.contains(&"$env:MY_VAR".to_string()));
}

/// Test that system variables are filtered from variable references
#[test]
fn test_filter_system_variables() {
    let path = "tests/fixtures/pipeline_with_filtering.yml";
    let var_refs = extract_variable_references(path).expect("Failed to extract variable references");

    // Should NOT contain system variables
    assert!(!var_refs.contains(&"Build.BuildNumber".to_string()));
    assert!(!var_refs.contains(&"System.DefaultWorkingDirectory".to_string()));
    assert!(!var_refs.contains(&"Agent.MachineName".to_string()));
    assert!(!var_refs.contains(&"Pipeline.Workspace".to_string()));
}

/// Test that runtime output variables are filtered from variable references
#[test]
fn test_filter_runtime_output_variables() {
    let path = "tests/fixtures/pipeline_with_filtering.yml";
    let var_refs = extract_variable_references(path).expect("Failed to extract variable references");

    // Should NOT contain runtime output variables
    assert!(!var_refs.contains(&"outputs.registryName".to_string()));
    assert!(!var_refs.contains(&"agentIp.value".to_string()));
    assert!(!var_refs.contains(&"domains.domainId".to_string()));
}

/// Test that regular variables are still extracted after filtering
#[test]
fn test_regular_variables_extracted_with_filtering() {
    let path = "tests/fixtures/pipeline_with_filtering.yml";
    let var_refs = extract_variable_references(path).expect("Failed to extract variable references");

    // Should contain regular custom variables
    assert!(var_refs.contains(&"customVar".to_string()));
    assert!(var_refs.contains(&"ApiKey".to_string()));
    assert!(var_refs.contains(&"ConnectionString".to_string()));

    // Should have exactly 3 variables (all the regular ones)
    assert_eq!(var_refs.len(), 3, "Should only have 3 regular variables after filtering");
}
