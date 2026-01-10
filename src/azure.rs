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
