//! Azure CLI integration for Azure DevOps API calls

use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::process::Command;

/// Variable group data returned from Azure DevOps
#[derive(Debug, Deserialize)]
pub struct VariableGroupData {
    /// Variable group ID
    pub id: i32,
    /// Variable group name
    pub name: String,
    /// Variables in the group (key = variable name, value = variable data)
    #[serde(default)]
    pub variables: HashMap<String, VariableValue>,
}

/// Variable value from a variable group
#[derive(Debug, Deserialize)]
pub struct VariableValue {
    /// The variable value (may be None for secret variables)
    pub value: Option<String>,
    /// Whether the variable is secret
    #[serde(rename = "isSecret", default)]
    pub is_secret: bool,
}

/// Client for interacting with Azure DevOps via Azure CLI
pub struct AzureDevOpsClient {
    /// Azure DevOps organization URL or name
    pub organization: String,
    /// Azure DevOps project name
    pub project: String,
}

impl AzureDevOpsClient {
    /// Create a new Azure DevOps client
    ///
    /// # Arguments
    /// * `organization` - Azure DevOps organization URL or name
    /// * `project` - Azure DevOps project name
    pub fn new(organization: String, project: String) -> Self {
        Self {
            organization,
            project,
        }
    }

    /// Check if Azure CLI is available and configured
    ///
    /// Runs 'az --version' to verify Azure CLI is installed
    ///
    /// # Returns
    /// * `Result<()>` - Ok if CLI is available, Error otherwise
    pub fn check_cli_available(&self) -> Result<()> {
        let output = Command::new("az")
            .arg("--version")
            .output()
            .with_context(|| "Failed to execute 'az --version'. Is Azure CLI installed?")?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Azure CLI check failed: {}", stderr)
        }
    }

    /// Fetch a variable group from Azure DevOps by name
    ///
    /// # Arguments
    /// * `group_name` - The name of the variable group to fetch
    ///
    /// # Returns
    /// * `Result<VariableGroupData>` - The variable group data if found
    pub fn get_variable_group(&self, group_name: &str) -> Result<VariableGroupData> {
        // Use 'az pipelines variable-group list' with a filter query
        let output = Command::new("az")
            .args([
                "pipelines",
                "variable-group",
                "list",
                "--organization",
                &self.organization,
                "--project",
                &self.project,
                "--query",
                &format!("[?name=='{}'] | [0]", group_name),
                "--output",
                "json",
            ])
            .output()
            .with_context(|| {
                format!(
                    "Failed to execute 'az pipelines variable-group list' for group '{}'",
                    group_name
                )
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!(
                "Azure CLI command failed for variable group '{}': {}",
                group_name,
                stderr
            );
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let trimmed = stdout.trim();

        // Check if the result is null or empty (group not found)
        if trimmed.is_empty() || trimmed == "null" {
            anyhow::bail!("Variable group '{}' not found", group_name);
        }

        // Parse the JSON response
        let group_data: VariableGroupData = serde_json::from_str(trimmed).with_context(|| {
            format!(
                "Failed to parse Azure CLI response for variable group '{}'",
                group_name
            )
        })?;

        Ok(group_data)
    }

    /// Get all variable names from a variable group by ID
    ///
    /// # Arguments
    /// * `group_id` - The ID of the variable group
    ///
    /// # Returns
    /// * `Result<Vec<String>>` - List of variable names in the group
    pub fn get_variables_in_group(&self, group_id: i32) -> Result<Vec<String>> {
        // Use 'az pipelines variable-group show' with the group ID
        let output = Command::new("az")
            .args([
                "pipelines",
                "variable-group",
                "show",
                "--id",
                &group_id.to_string(),
                "--organization",
                &self.organization,
                "--project",
                &self.project,
                "--output",
                "json",
            ])
            .output()
            .with_context(|| {
                format!(
                    "Failed to execute 'az pipelines variable-group show' for group ID {}",
                    group_id
                )
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!(
                "Azure CLI command failed for variable group ID {}: {}",
                group_id,
                stderr
            );
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let trimmed = stdout.trim();

        // Check if the result is null or empty (group not found)
        if trimmed.is_empty() || trimmed == "null" {
            anyhow::bail!("Variable group with ID {} not found", group_id);
        }

        // Parse the JSON response
        let group_data: VariableGroupData = serde_json::from_str(trimmed).with_context(|| {
            format!(
                "Failed to parse Azure CLI response for variable group ID {}",
                group_id
            )
        })?;

        // Extract variable names from the variables HashMap
        let variable_names: Vec<String> = group_data.variables.keys().cloned().collect();

        Ok(variable_names)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = AzureDevOpsClient::new(
            "https://dev.azure.com/myorg".to_string(),
            "myproject".to_string(),
        );

        assert_eq!(client.organization, "https://dev.azure.com/myorg");
        assert_eq!(client.project, "myproject");
    }

    #[test]
    fn test_client_creation_with_org_name() {
        let client = AzureDevOpsClient::new("myorg".to_string(), "myproject".to_string());

        assert_eq!(client.organization, "myorg");
        assert_eq!(client.project, "myproject");
    }

    #[test]
    fn test_parse_variable_group_response() {
        // Sample Azure CLI response for variable group
        let json_response = r#"{
            "id": 123,
            "name": "ProductionSecrets",
            "variables": {
                "ConnectionString": {
                    "value": "Server=prod.db;",
                    "isSecret": false
                },
                "ApiKey": {
                    "value": null,
                    "isSecret": true
                }
            }
        }"#;

        let group_data: VariableGroupData =
            serde_json::from_str(json_response).expect("Failed to parse JSON");

        assert_eq!(group_data.id, 123);
        assert_eq!(group_data.name, "ProductionSecrets");
        assert_eq!(group_data.variables.len(), 2);

        // Verify ConnectionString variable
        let conn_string = group_data
            .variables
            .get("ConnectionString")
            .expect("ConnectionString not found");
        assert_eq!(conn_string.value, Some("Server=prod.db;".to_string()));
        assert!(!conn_string.is_secret);

        // Verify ApiKey variable (secret)
        let api_key = group_data.variables.get("ApiKey").expect("ApiKey not found");
        assert!(api_key.value.is_none());
        assert!(api_key.is_secret);
    }

    #[test]
    fn test_parse_variable_group_empty_variables() {
        // Sample response with no variables
        let json_response = r#"{
            "id": 456,
            "name": "EmptyGroup"
        }"#;

        let group_data: VariableGroupData =
            serde_json::from_str(json_response).expect("Failed to parse JSON");

        assert_eq!(group_data.id, 456);
        assert_eq!(group_data.name, "EmptyGroup");
        assert!(group_data.variables.is_empty());
    }

    #[test]
    fn test_parse_variable_group_with_missing_optional_fields() {
        // Response where isSecret is missing (should default to false)
        let json_response = r#"{
            "id": 789,
            "name": "TestGroup",
            "variables": {
                "SimpleVar": {
                    "value": "hello"
                }
            }
        }"#;

        let group_data: VariableGroupData =
            serde_json::from_str(json_response).expect("Failed to parse JSON");

        let simple_var = group_data
            .variables
            .get("SimpleVar")
            .expect("SimpleVar not found");
        assert_eq!(simple_var.value, Some("hello".to_string()));
        assert!(!simple_var.is_secret); // Default value
    }

    #[test]
    fn test_extract_variable_names_from_group_data() {
        let json_response = r#"{
            "id": 100,
            "name": "MyGroup",
            "variables": {
                "Var1": {"value": "a"},
                "Var2": {"value": "b"},
                "Var3": {"value": "c"}
            }
        }"#;

        let group_data: VariableGroupData =
            serde_json::from_str(json_response).expect("Failed to parse JSON");

        let mut variable_names: Vec<String> = group_data.variables.keys().cloned().collect();
        variable_names.sort(); // Sort for consistent comparison

        assert_eq!(variable_names.len(), 3);
        assert_eq!(variable_names, vec!["Var1", "Var2", "Var3"]);
    }

    #[test]
    fn test_variable_value_deserialization_with_null_value() {
        let json_response = r#"{
            "value": null,
            "isSecret": true
        }"#;

        let var_value: VariableValue =
            serde_json::from_str(json_response).expect("Failed to parse JSON");

        assert!(var_value.value.is_none());
        assert!(var_value.is_secret);
    }

    #[test]
    fn test_variable_value_deserialization_minimal() {
        // Minimal response with only value
        let json_response = r#"{"value": "test-value"}"#;

        let var_value: VariableValue =
            serde_json::from_str(json_response).expect("Failed to parse JSON");

        assert_eq!(var_value.value, Some("test-value".to_string()));
        assert!(!var_value.is_secret);
    }
}
