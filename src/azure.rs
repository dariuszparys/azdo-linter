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
    /// Whether the variable is secret (can be null in Azure DevOps API response)
    #[serde(rename = "isSecret")]
    pub is_secret: Option<bool>,
}

/// Variable data from a pipeline definition
#[derive(Debug, Deserialize)]
pub struct PipelineVariableValue {
    /// The variable value (may be None for secret variables)
    pub value: Option<String>,
    /// Whether the variable is secret (can be null in Azure DevOps API response)
    #[serde(rename = "isSecret")]
    pub is_secret: Option<bool>,
    /// Whether the variable can be overridden at queue time
    #[serde(rename = "allowOverride", default)]
    pub allow_override: bool,
}

/// Pipeline info from az pipelines list
#[derive(Debug, Deserialize)]
pub struct PipelineInfo {
    /// Pipeline ID
    pub id: i32,
    /// Pipeline name
    pub name: String,
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
        // Normalize organization to full URL if needed
        let organization_url = if organization.starts_with("https://") || organization.starts_with("http://") {
            organization
        } else {
            format!("https://dev.azure.com/{organization}")
        };

        Self {
            organization: organization_url,
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
            anyhow::bail!("Azure CLI check failed: {stderr}")
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
                &format!("[?name=='{group_name}'] | [0]"),
                "--output",
                "json",
            ])
            .output()
            .with_context(|| {
                format!(
                    "Failed to execute 'az pipelines variable-group list' for group '{group_name}'"
                )
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!(
                "Azure CLI command failed for variable group '{group_name}': {stderr}"
            );
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let trimmed = stdout.trim();

        // Check if the result is null or empty (group not found)
        if trimmed.is_empty() || trimmed == "null" {
            anyhow::bail!("Variable group '{group_name}' not found");
        }

        // Parse the JSON response
        let group_data: VariableGroupData = serde_json::from_str(trimmed).with_context(|| {
            format!(
                "Failed to parse Azure CLI response for variable group '{group_name}'"
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
                    "Failed to execute 'az pipelines variable-group show' for group ID {group_id}"
                )
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!(
                "Azure CLI command failed for variable group ID {group_id}: {stderr}"
            );
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let trimmed = stdout.trim();

        // Check if the result is null or empty (group not found)
        if trimmed.is_empty() || trimmed == "null" {
            anyhow::bail!("Variable group with ID {group_id} not found");
        }

        // Parse the JSON response
        let group_data: VariableGroupData = serde_json::from_str(trimmed).with_context(|| {
            format!(
                "Failed to parse Azure CLI response for variable group ID {group_id}"
            )
        })?;

        // Extract variable names from the variables HashMap
        let variable_names: Vec<String> = group_data.variables.keys().cloned().collect();

        Ok(variable_names)
    }

    /// Look up a pipeline ID by name
    ///
    /// Uses `az pipelines list --name` to find the pipeline, then extracts the ID.
    /// This works around a bug in `az pipelines variable list --pipeline-name` which
    /// fails to find pipelines by name, while `--pipeline-id` works correctly.
    ///
    /// # Arguments
    /// * `pipeline_name` - The name of the pipeline
    ///
    /// # Returns
    /// * `Result<i32>` - The pipeline ID if found
    pub fn get_pipeline_id_by_name(&self, pipeline_name: &str) -> Result<i32> {
        // Note: We don't use --name filter because it doesn't work reliably
        // (returns empty results even for exact matches). Instead, we list all
        // pipelines and filter locally.
        let output = Command::new("az")
            .args([
                "pipelines",
                "list",
                "--organization",
                &self.organization,
                "--project",
                &self.project,
                "--output",
                "json",
            ])
            .output()
            .with_context(|| {
                format!("Failed to execute 'az pipelines list' for pipeline '{pipeline_name}'")
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Azure CLI command failed when looking up pipeline '{pipeline_name}': {stderr}");
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let trimmed = stdout.trim();

        // Check for empty response
        if trimmed.is_empty() || trimmed == "[]" || trimmed == "null" {
            anyhow::bail!("Pipeline '{pipeline_name}' not found");
        }

        // Parse the JSON response (array of pipelines)
        let pipelines: Vec<PipelineInfo> = serde_json::from_str(trimmed).with_context(|| {
            format!("Failed to parse Azure CLI response when looking up pipeline '{pipeline_name}'")
        })?;

        // Find exact match by name
        let pipeline = pipelines
            .iter()
            .find(|p| p.name == pipeline_name)
            .ok_or_else(|| anyhow::anyhow!("Pipeline '{pipeline_name}' not found"))?;

        Ok(pipeline.id)
    }

    /// Fetch pipeline definition variables from Azure DevOps by name
    ///
    /// First resolves the pipeline name to an ID using `az pipelines list`,
    /// then fetches variables using the ID. This works around a bug in the
    /// Azure CLI where `--pipeline-name` doesn't work for `az pipelines variable list`.
    ///
    /// # Arguments
    /// * `pipeline_name` - The name of the pipeline
    ///
    /// # Returns
    /// * `Result<HashMap<String, PipelineVariableValue>>` - Map of variable name to value
    pub fn get_pipeline_variables(
        &self,
        pipeline_name: &str,
    ) -> Result<HashMap<String, PipelineVariableValue>> {
        // First resolve name to ID (works around Azure CLI bug)
        let pipeline_id = self.get_pipeline_id_by_name(pipeline_name)?;

        // Then fetch variables using the ID (which works reliably)
        self.get_pipeline_variables_by_id(pipeline_id)
    }

    /// Get variable names from a pipeline definition
    ///
    /// # Arguments
    /// * `pipeline_name` - The name of the pipeline
    ///
    /// # Returns
    /// * `Result<Vec<String>>` - List of variable names defined on the pipeline
    pub fn get_pipeline_variable_names(&self, pipeline_name: &str) -> Result<Vec<String>> {
        let variables = self.get_pipeline_variables(pipeline_name)?;
        Ok(variables.keys().cloned().collect())
    }

    /// Fetch pipeline definition variables from Azure DevOps by pipeline ID
    ///
    /// # Arguments
    /// * `pipeline_id` - The ID of the pipeline (more reliable than name)
    ///
    /// # Returns
    /// * `Result<HashMap<String, PipelineVariableValue>>` - Map of variable name to value
    pub fn get_pipeline_variables_by_id(
        &self,
        pipeline_id: i32,
    ) -> Result<HashMap<String, PipelineVariableValue>> {
        let output = Command::new("az")
            .args([
                "pipelines",
                "variable",
                "list",
                "--pipeline-id",
                &pipeline_id.to_string(),
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
                    "Failed to execute 'az pipelines variable list' for pipeline ID {pipeline_id}"
                )
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Azure CLI command failed for pipeline ID {pipeline_id}: {stderr}");
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let trimmed = stdout.trim();

        // Check for empty response (pipeline has no variables or doesn't exist)
        if trimmed.is_empty() || trimmed == "{}" || trimmed == "null" {
            return Ok(HashMap::new());
        }

        // Parse the JSON response
        let variables: HashMap<String, PipelineVariableValue> =
            serde_json::from_str(trimmed).with_context(|| {
                format!("Failed to parse Azure CLI response for pipeline ID {pipeline_id}")
            })?;

        Ok(variables)
    }

    /// Get variable names from a pipeline definition by ID
    ///
    /// # Arguments
    /// * `pipeline_id` - The ID of the pipeline
    ///
    /// # Returns
    /// * `Result<Vec<String>>` - List of variable names defined on the pipeline
    pub fn get_pipeline_variable_names_by_id(&self, pipeline_id: i32) -> Result<Vec<String>> {
        let variables = self.get_pipeline_variables_by_id(pipeline_id)?;
        Ok(variables.keys().cloned().collect())
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

        // Organization name should be normalized to full URL
        assert_eq!(client.organization, "https://dev.azure.com/myorg");
        assert_eq!(client.project, "myproject");
    }

    #[test]
    fn test_client_creation_preserves_full_url() {
        let client = AzureDevOpsClient::new(
            "https://dev.azure.com/customorg".to_string(),
            "myproject".to_string(),
        );

        // Full URL should be preserved as-is
        assert_eq!(client.organization, "https://dev.azure.com/customorg");
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
        assert_eq!(conn_string.is_secret, Some(false));

        // Verify ApiKey variable (secret)
        let api_key = group_data.variables.get("ApiKey").expect("ApiKey not found");
        assert!(api_key.value.is_none());
        assert_eq!(api_key.is_secret, Some(true));
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
        assert_eq!(simple_var.is_secret, None); // Missing field becomes None
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
        assert_eq!(var_value.is_secret, Some(true));
    }

    #[test]
    fn test_variable_value_deserialization_minimal() {
        // Minimal response with only value
        let json_response = r#"{"value": "test-value"}"#;

        let var_value: VariableValue =
            serde_json::from_str(json_response).expect("Failed to parse JSON");

        assert_eq!(var_value.value, Some("test-value".to_string()));
        assert_eq!(var_value.is_secret, None); // Missing field becomes None
    }

    #[test]
    fn test_variable_value_deserialization_with_null_is_secret() {
        // Test that null isSecret (common in Azure DevOps responses) works
        let json_response = r#"{"value": "test-value", "isSecret": null}"#;

        let var_value: VariableValue =
            serde_json::from_str(json_response).expect("Failed to parse JSON");

        assert_eq!(var_value.value, Some("test-value".to_string()));
        assert_eq!(var_value.is_secret, None); // Explicit null becomes None
    }

    // Tests for PipelineVariableValue

    #[test]
    fn test_parse_pipeline_variables_response() {
        let json_response = r#"{
            "varName": {
                "value": "the-value",
                "isSecret": false,
                "allowOverride": true
            },
            "secretVar": {
                "value": null,
                "isSecret": true,
                "allowOverride": false
            }
        }"#;

        let variables: HashMap<String, PipelineVariableValue> =
            serde_json::from_str(json_response).expect("Failed to parse JSON");

        assert_eq!(variables.len(), 2);
        assert!(variables.contains_key("varName"));
        assert!(variables.contains_key("secretVar"));

        let var = variables.get("varName").unwrap();
        assert_eq!(var.value, Some("the-value".to_string()));
        assert_eq!(var.is_secret, Some(false));
        assert!(var.allow_override);

        let secret = variables.get("secretVar").unwrap();
        assert!(secret.value.is_none());
        assert_eq!(secret.is_secret, Some(true));
        assert!(!secret.allow_override);
    }

    #[test]
    fn test_parse_empty_pipeline_variables() {
        let json_response = "{}";
        let variables: HashMap<String, PipelineVariableValue> =
            serde_json::from_str(json_response).expect("Failed to parse JSON");
        assert!(variables.is_empty());
    }

    #[test]
    fn test_parse_pipeline_variable_minimal() {
        // Minimal response with only value (other fields should default)
        let json_response = r#"{
            "minimalVar": {
                "value": "test"
            }
        }"#;

        let variables: HashMap<String, PipelineVariableValue> =
            serde_json::from_str(json_response).expect("Failed to parse JSON");

        let var = variables.get("minimalVar").unwrap();
        assert_eq!(var.value, Some("test".to_string()));
        assert_eq!(var.is_secret, None); // missing field becomes None
        assert!(!var.allow_override); // defaults to false
    }

    #[test]
    fn test_parse_pipeline_variable_with_null_is_secret() {
        // Test that null isSecret (actual Azure DevOps response format) works
        let json_response = r#"{
            "helloPipeline": {
                "allowOverride": true,
                "isSecret": null,
                "value": "I'm a pipeline variable"
            }
        }"#;

        let variables: HashMap<String, PipelineVariableValue> =
            serde_json::from_str(json_response).expect("Failed to parse JSON");

        let var = variables.get("helloPipeline").unwrap();
        assert_eq!(var.value, Some("I'm a pipeline variable".to_string()));
        assert_eq!(var.is_secret, None); // null becomes None
        assert!(var.allow_override);
    }
}
