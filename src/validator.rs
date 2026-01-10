//! Validation logic for pipeline variable groups and variables

use anyhow::Result;
use crate::azure::AzureDevOpsClient;

/// Result of validating a single variable group
#[derive(Debug)]
pub struct GroupValidationResult {
    /// Name of the variable group
    pub group_name: String,
    /// Whether the group exists in Azure DevOps
    pub exists: bool,
    /// Optional error message if validation failed
    pub error: Option<String>,
    /// Variable group ID if found
    pub group_id: Option<i32>,
}

/// Source of a validated variable
#[derive(Debug, Clone, PartialEq)]
pub enum VariableSource {
    /// Variable found in a variable group
    Group(String),
    /// Variable defined inline in the pipeline
    Inline,
    /// Variable not found
    NotFound,
}

/// Result of validating a single variable reference
#[derive(Debug)]
pub struct VariableValidationResult {
    /// Name of the variable being validated
    pub variable_name: String,
    /// Name of the variable group where it was found (if any)
    pub group_name: Option<String>,
    /// Whether the variable exists in any of the referenced groups
    pub exists: bool,
    /// Optional error message if validation failed
    pub error: Option<String>,
    /// Source of the variable (group, inline, or not found)
    pub source: VariableSource,
}

/// Validate that variable groups exist in Azure DevOps
///
/// # Arguments
/// * `group_names` - List of variable group names to validate
/// * `client` - Azure DevOps client for API calls
///
/// # Returns
/// * `Result<Vec<GroupValidationResult>>` - Validation results for each group
pub fn validate_variable_groups(
    group_names: Vec<String>,
    client: &AzureDevOpsClient,
) -> Result<Vec<GroupValidationResult>> {
    let mut results = Vec::new();

    for group_name in group_names {
        let result = match client.get_variable_group(&group_name) {
            Ok(group_data) => GroupValidationResult {
                group_name,
                exists: true,
                error: None,
                group_id: Some(group_data.id),
            },
            Err(e) => GroupValidationResult {
                group_name,
                exists: false,
                error: Some(e.to_string()),
                group_id: None,
            },
        };
        results.push(result);
    }

    Ok(results)
}

/// Validate that variables referenced in the pipeline exist in the variable groups or are defined inline
///
/// # Arguments
/// * `variable_references` - List of variable names referenced in the pipeline (using $(variableName) syntax)
/// * `group_validation_results` - Results from validating variable groups (contains group IDs)
/// * `inline_variables` - List of variable names defined inline in the pipeline
/// * `client` - Azure DevOps client for API calls
///
/// # Returns
/// * `Result<Vec<VariableValidationResult>>` - Validation results for each variable
pub fn validate_variables(
    variable_references: Vec<String>,
    group_validation_results: &[GroupValidationResult],
    inline_variables: &[String],
    client: &AzureDevOpsClient,
) -> Result<Vec<VariableValidationResult>> {
    // Collect all available variables from all existing groups
    let mut available_variables: Vec<(String, String)> = Vec::new(); // (variable_name, group_name)

    for group_result in group_validation_results {
        if group_result.exists {
            if let Some(group_id) = group_result.group_id {
                match client.get_variables_in_group(group_id) {
                    Ok(vars) => {
                        for var in vars {
                            available_variables.push((var, group_result.group_name.clone()));
                        }
                    }
                    Err(_) => {
                        // Skip groups that fail to fetch variables - already reported in group validation
                    }
                }
            }
        }
    }

    // Validate each variable reference
    let mut results = Vec::new();

    for var_name in variable_references {
        // First check if it's an inline variable
        if inline_variables.contains(&var_name) {
            results.push(VariableValidationResult {
                variable_name: var_name,
                group_name: None,
                exists: true,
                error: None,
                source: VariableSource::Inline,
            });
            continue;
        }

        // Search for the variable in all available groups
        let found = available_variables
            .iter()
            .find(|(name, _)| name == &var_name);

        let result = match found {
            Some((_, group_name)) => VariableValidationResult {
                variable_name: var_name,
                group_name: Some(group_name.clone()),
                exists: true,
                error: None,
                source: VariableSource::Group(group_name.clone()),
            },
            None => VariableValidationResult {
                variable_name: var_name,
                group_name: None,
                exists: false,
                error: Some("Variable not found in any referenced variable group".to_string()),
                source: VariableSource::NotFound,
            },
        };
        results.push(result);
    }

    Ok(results)
}

/// Helper function to validate variables against pre-fetched available variables
/// This is used for testing without needing to call Azure CLI
pub fn validate_variables_against_available(
    variable_references: Vec<String>,
    available_variables: &[(String, String)], // (variable_name, group_name)
) -> Vec<VariableValidationResult> {
    validate_variables_against_available_with_inline(variable_references, available_variables, &[])
}

/// Helper function to validate variables against pre-fetched available variables and inline variables
/// This is used for testing without needing to call Azure CLI
pub fn validate_variables_against_available_with_inline(
    variable_references: Vec<String>,
    available_variables: &[(String, String)], // (variable_name, group_name)
    inline_variables: &[String],
) -> Vec<VariableValidationResult> {
    let mut results = Vec::new();

    for var_name in variable_references {
        // First check if it's an inline variable
        if inline_variables.contains(&var_name) {
            results.push(VariableValidationResult {
                variable_name: var_name,
                group_name: None,
                exists: true,
                error: None,
                source: VariableSource::Inline,
            });
            continue;
        }

        let found = available_variables
            .iter()
            .find(|(name, _)| name == &var_name);

        let result = match found {
            Some((_, group_name)) => VariableValidationResult {
                variable_name: var_name,
                group_name: Some(group_name.clone()),
                exists: true,
                error: None,
                source: VariableSource::Group(group_name.clone()),
            },
            None => VariableValidationResult {
                variable_name: var_name,
                group_name: None,
                exists: false,
                error: Some("Variable not found in any referenced variable group".to_string()),
                source: VariableSource::NotFound,
            },
        };
        results.push(result);
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests for GroupValidationResult struct
    #[test]
    fn test_group_validation_result_exists() {
        let result = GroupValidationResult {
            group_name: "MyGroup".to_string(),
            exists: true,
            error: None,
            group_id: Some(123),
        };

        assert_eq!(result.group_name, "MyGroup");
        assert!(result.exists);
        assert!(result.error.is_none());
        assert_eq!(result.group_id, Some(123));
    }

    #[test]
    fn test_group_validation_result_not_found() {
        let result = GroupValidationResult {
            group_name: "MissingGroup".to_string(),
            exists: false,
            error: Some("Group not found".to_string()),
            group_id: None,
        };

        assert_eq!(result.group_name, "MissingGroup");
        assert!(!result.exists);
        assert_eq!(result.error, Some("Group not found".to_string()));
        assert!(result.group_id.is_none());
    }

    // Tests for VariableValidationResult struct
    #[test]
    fn test_variable_validation_result_found() {
        let result = VariableValidationResult {
            variable_name: "ApiKey".to_string(),
            group_name: Some("Secrets".to_string()),
            exists: true,
            error: None,
            source: VariableSource::Group("Secrets".to_string()),
        };

        assert_eq!(result.variable_name, "ApiKey");
        assert_eq!(result.group_name, Some("Secrets".to_string()));
        assert!(result.exists);
        assert!(result.error.is_none());
        assert_eq!(result.source, VariableSource::Group("Secrets".to_string()));
    }

    #[test]
    fn test_variable_validation_result_not_found() {
        let result = VariableValidationResult {
            variable_name: "MissingVar".to_string(),
            group_name: None,
            exists: false,
            error: Some("Variable not found".to_string()),
            source: VariableSource::NotFound,
        };

        assert_eq!(result.variable_name, "MissingVar");
        assert!(result.group_name.is_none());
        assert!(!result.exists);
        assert!(result.error.is_some());
        assert_eq!(result.source, VariableSource::NotFound);
    }

    #[test]
    fn test_variable_validation_result_inline() {
        let result = VariableValidationResult {
            variable_name: "BuildConfig".to_string(),
            group_name: None,
            exists: true,
            error: None,
            source: VariableSource::Inline,
        };

        assert_eq!(result.variable_name, "BuildConfig");
        assert!(result.group_name.is_none());
        assert!(result.exists);
        assert!(result.error.is_none());
        assert_eq!(result.source, VariableSource::Inline);
    }

    // Tests for validate_variables_against_available function
    #[test]
    fn test_validate_all_variables_exist() {
        let available = vec![
            ("Var1".to_string(), "Group1".to_string()),
            ("Var2".to_string(), "Group1".to_string()),
            ("Var3".to_string(), "Group2".to_string()),
        ];

        let references = vec![
            "Var1".to_string(),
            "Var2".to_string(),
            "Var3".to_string(),
        ];

        let results = validate_variables_against_available(references, &available);

        assert_eq!(results.len(), 3);
        assert!(results.iter().all(|r| r.exists));
        assert!(results.iter().all(|r| r.error.is_none()));

        // Check specific mappings
        assert_eq!(results[0].group_name, Some("Group1".to_string()));
        assert_eq!(results[1].group_name, Some("Group1".to_string()));
        assert_eq!(results[2].group_name, Some("Group2".to_string()));
    }

    #[test]
    fn test_validate_some_variables_missing() {
        let available = vec![
            ("Var1".to_string(), "Group1".to_string()),
            ("Var2".to_string(), "Group1".to_string()),
        ];

        let references = vec![
            "Var1".to_string(),
            "MissingVar".to_string(),
            "Var2".to_string(),
        ];

        let results = validate_variables_against_available(references, &available);

        assert_eq!(results.len(), 3);

        // First variable should exist
        assert!(results[0].exists);
        assert_eq!(results[0].variable_name, "Var1");

        // Second variable should be missing
        assert!(!results[1].exists);
        assert_eq!(results[1].variable_name, "MissingVar");
        assert!(results[1].error.is_some());

        // Third variable should exist
        assert!(results[2].exists);
        assert_eq!(results[2].variable_name, "Var2");
    }

    #[test]
    fn test_validate_no_variables_exist() {
        let available = vec![
            ("Var1".to_string(), "Group1".to_string()),
        ];

        let references = vec![
            "Missing1".to_string(),
            "Missing2".to_string(),
        ];

        let results = validate_variables_against_available(references, &available);

        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| !r.exists));
        assert!(results.iter().all(|r| r.group_name.is_none()));
    }

    #[test]
    fn test_validate_empty_variable_references() {
        let available = vec![
            ("Var1".to_string(), "Group1".to_string()),
        ];

        let references: Vec<String> = vec![];

        let results = validate_variables_against_available(references, &available);

        assert!(results.is_empty());
    }

    #[test]
    fn test_validate_empty_available_variables() {
        let available: Vec<(String, String)> = vec![];

        let references = vec![
            "Var1".to_string(),
            "Var2".to_string(),
        ];

        let results = validate_variables_against_available(references, &available);

        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| !r.exists));
    }

    #[test]
    fn test_validate_variable_in_multiple_groups() {
        // Same variable name in multiple groups - should find the first one
        let available = vec![
            ("SharedVar".to_string(), "Group1".to_string()),
            ("SharedVar".to_string(), "Group2".to_string()),
        ];

        let references = vec!["SharedVar".to_string()];

        let results = validate_variables_against_available(references, &available);

        assert_eq!(results.len(), 1);
        assert!(results[0].exists);
        // Should find the first occurrence (Group1)
        assert_eq!(results[0].group_name, Some("Group1".to_string()));
    }

    #[test]
    fn test_validate_case_sensitive_matching() {
        let available = vec![
            ("ConnectionString".to_string(), "Group1".to_string()),
        ];

        let references = vec![
            "ConnectionString".to_string(),
            "connectionstring".to_string(), // Different case
        ];

        let results = validate_variables_against_available(references, &available);

        assert_eq!(results.len(), 2);
        assert!(results[0].exists); // Exact match
        assert!(!results[1].exists); // Case mismatch - not found
    }

    #[test]
    fn test_validate_inline_variables() {
        let available = vec![
            ("GroupVar".to_string(), "Group1".to_string()),
        ];

        let inline = vec![
            "InlineVar1".to_string(),
            "InlineVar2".to_string(),
        ];

        let references = vec![
            "GroupVar".to_string(),
            "InlineVar1".to_string(),
            "MissingVar".to_string(),
        ];

        let results = validate_variables_against_available_with_inline(references, &available, &inline);

        assert_eq!(results.len(), 3);

        // First variable should be from group
        assert!(results[0].exists);
        assert_eq!(results[0].source, VariableSource::Group("Group1".to_string()));

        // Second variable should be inline
        assert!(results[1].exists);
        assert_eq!(results[1].source, VariableSource::Inline);

        // Third variable should be missing
        assert!(!results[2].exists);
        assert_eq!(results[2].source, VariableSource::NotFound);
    }

    #[test]
    fn test_inline_takes_precedence_over_group() {
        // If a variable is both inline and in a group, inline should take precedence
        let available = vec![
            ("SharedVar".to_string(), "Group1".to_string()),
        ];

        let inline = vec![
            "SharedVar".to_string(),
        ];

        let references = vec!["SharedVar".to_string()];

        let results = validate_variables_against_available_with_inline(references, &available, &inline);

        assert_eq!(results.len(), 1);
        assert!(results[0].exists);
        // Should be marked as inline, not group
        assert_eq!(results[0].source, VariableSource::Inline);
    }
}
