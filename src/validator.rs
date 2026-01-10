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
