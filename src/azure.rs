//! Azure DevOps REST API client

use anyhow::{Context, Result};
use base64::Engine;
use reqwest::blocking::Client;
use reqwest::header::{HeaderValue, ACCEPT, AUTHORIZATION};
use serde::Deserialize;
use std::collections::HashMap;

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

/// Pipeline info from pipelines list
#[derive(Debug, Deserialize)]
pub struct PipelineInfo {
    /// Pipeline ID
    pub id: i32,
    /// Pipeline name
    pub name: String,
}

/// Response wrapper for variable groups list endpoint
#[derive(Debug, Deserialize)]
struct VariableGroupsResponse {
    #[serde(default)]
    value: Vec<VariableGroupData>,
}

/// Response wrapper for pipelines list endpoint
#[derive(Debug, Deserialize)]
struct PipelinesResponse {
    #[serde(default)]
    value: Vec<PipelineInfo>,
}

/// Build definition response (contains variables)
#[derive(Debug, Deserialize)]
struct BuildDefinitionResponse {
    #[serde(default)]
    variables: HashMap<String, PipelineVariableValue>,
}

/// Client for interacting with Azure DevOps via REST API
#[derive(Debug)]
pub struct AzureDevOpsClient {
    /// Azure DevOps organization URL
    pub organization: String,
    /// Azure DevOps project name
    pub project: String,
    /// HTTP client
    http_client: Client,
    /// Authorization header value (pre-computed Basic auth)
    auth_header: HeaderValue,
}

impl AzureDevOpsClient {
    /// Create a new Azure DevOps client with PAT authentication
    ///
    /// # Arguments
    /// * `organization` - Azure DevOps organization URL or name
    /// * `project` - Azure DevOps project name
    /// * `pat` - Personal Access Token for authentication (optional, falls back to AZDO_PAT env var)
    ///
    /// # Returns
    /// * `Result<Self>` - The client or an error if PAT is missing
    pub fn new(organization: String, project: String, pat: Option<String>) -> Result<Self> {
        // Normalize organization to full URL if needed
        let organization_url =
            if organization.starts_with("https://") || organization.starts_with("http://") {
                organization
            } else {
                format!("https://dev.azure.com/{organization}")
            };

        // Get PAT from argument or environment variable
        let pat_value = pat.or_else(|| std::env::var("AZDO_PAT").ok()).ok_or_else(|| {
            anyhow::anyhow!(
                "No authentication token provided. Set AZDO_PAT environment variable or use --pat argument."
            )
        })?;

        // Create auth header: Basic base64(":" + PAT)
        // Azure DevOps uses empty username with PAT as password
        let auth_string = format!(":{}", pat_value);
        let encoded = base64::engine::general_purpose::STANDARD.encode(auth_string.as_bytes());
        let auth_header = HeaderValue::from_str(&format!("Basic {}", encoded))
            .context("Failed to create authorization header")?;

        let http_client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self {
            organization: organization_url,
            project,
            http_client,
            auth_header,
        })
    }

    /// Construct the project URL base
    fn project_url(&self) -> String {
        format!("{}/{}", self.organization, self.project)
    }

    /// Handle HTTP response status codes with helpful error messages
    fn handle_response_error(
        &self,
        status: reqwest::StatusCode,
        context: &str,
    ) -> anyhow::Error {
        match status.as_u16() {
            401 => anyhow::anyhow!(
                "Authentication failed for {}. Check that your PAT is valid and not expired.",
                context
            ),
            403 => anyhow::anyhow!(
                "Access denied for {}. Check that your PAT has sufficient permissions (Variable Groups Read, Build Read).",
                context
            ),
            404 => anyhow::anyhow!("{} not found.", context),
            _ => anyhow::anyhow!(
                "HTTP {} error for {}: {}",
                status.as_u16(),
                context,
                status.canonical_reason().unwrap_or("Unknown error")
            ),
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
        let encoded_name = urlencoding::encode(group_name);
        let url = format!(
            "{}/_apis/distributedtask/variablegroups?groupName={}&api-version=7.0",
            self.project_url(),
            encoded_name
        );

        let response = self
            .http_client
            .get(&url)
            .header(AUTHORIZATION, self.auth_header.clone())
            .header(ACCEPT, "application/json")
            .send()
            .with_context(|| format!("Failed to send request for variable group '{}'", group_name))?;

        let status = response.status();
        if !status.is_success() {
            return Err(self.handle_response_error(
                status,
                &format!("variable group '{}'", group_name),
            ));
        }

        let groups_response: VariableGroupsResponse = response.json().with_context(|| {
            format!(
                "Failed to parse response for variable group '{}'",
                group_name
            )
        })?;

        // Find exact match by name (API may return partial matches)
        groups_response
            .value
            .into_iter()
            .find(|g| g.name == group_name)
            .ok_or_else(|| anyhow::anyhow!("Variable group '{}' not found", group_name))
    }

    /// Get all variable names from a variable group by ID
    ///
    /// # Arguments
    /// * `group_id` - The ID of the variable group
    ///
    /// # Returns
    /// * `Result<Vec<String>>` - List of variable names in the group
    pub fn get_variables_in_group(&self, group_id: i32) -> Result<Vec<String>> {
        let url = format!(
            "{}/_apis/distributedtask/variablegroups/{}?api-version=7.0",
            self.project_url(),
            group_id
        );

        let response = self
            .http_client
            .get(&url)
            .header(AUTHORIZATION, self.auth_header.clone())
            .header(ACCEPT, "application/json")
            .send()
            .with_context(|| {
                format!(
                    "Failed to send request for variable group ID {}",
                    group_id
                )
            })?;

        let status = response.status();
        if !status.is_success() {
            return Err(self.handle_response_error(
                status,
                &format!("variable group ID {}", group_id),
            ));
        }

        let group_data: VariableGroupData = response.json().with_context(|| {
            format!(
                "Failed to parse response for variable group ID {}",
                group_id
            )
        })?;

        Ok(group_data.variables.keys().cloned().collect())
    }

    /// Look up a pipeline ID by name
    ///
    /// # Arguments
    /// * `pipeline_name` - The name of the pipeline
    ///
    /// # Returns
    /// * `Result<i32>` - The pipeline ID if found
    pub fn get_pipeline_id_by_name(&self, pipeline_name: &str) -> Result<i32> {
        let url = format!("{}/_apis/pipelines?api-version=7.0", self.project_url());

        let response = self
            .http_client
            .get(&url)
            .header(AUTHORIZATION, self.auth_header.clone())
            .header(ACCEPT, "application/json")
            .send()
            .with_context(|| {
                format!(
                    "Failed to send request for pipeline '{}'",
                    pipeline_name
                )
            })?;

        let status = response.status();
        if !status.is_success() {
            return Err(self.handle_response_error(
                status,
                &format!("pipeline '{}'", pipeline_name),
            ));
        }

        let pipelines_response: PipelinesResponse = response.json().with_context(|| {
            format!(
                "Failed to parse response when looking up pipeline '{}'",
                pipeline_name
            )
        })?;

        // Find exact match by name
        pipelines_response
            .value
            .iter()
            .find(|p| p.name == pipeline_name)
            .map(|p| p.id)
            .ok_or_else(|| anyhow::anyhow!("Pipeline '{}' not found", pipeline_name))
    }

    /// Fetch pipeline definition variables from Azure DevOps by name
    ///
    /// First resolves the pipeline name to an ID, then fetches variables using the ID.
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
        let pipeline_id = self.get_pipeline_id_by_name(pipeline_name)?;
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
    /// * `pipeline_id` - The ID of the pipeline
    ///
    /// # Returns
    /// * `Result<HashMap<String, PipelineVariableValue>>` - Map of variable name to value
    pub fn get_pipeline_variables_by_id(
        &self,
        pipeline_id: i32,
    ) -> Result<HashMap<String, PipelineVariableValue>> {
        // Use build definitions API to get pipeline with variables
        let url = format!(
            "{}/_apis/build/definitions/{}?api-version=7.0",
            self.project_url(),
            pipeline_id
        );

        let response = self
            .http_client
            .get(&url)
            .header(AUTHORIZATION, self.auth_header.clone())
            .header(ACCEPT, "application/json")
            .send()
            .with_context(|| {
                format!(
                    "Failed to send request for pipeline ID {}",
                    pipeline_id
                )
            })?;

        let status = response.status();
        if !status.is_success() {
            return Err(self.handle_response_error(
                status,
                &format!("pipeline ID {}", pipeline_id),
            ));
        }

        let definition: BuildDefinitionResponse = response.json().with_context(|| {
            format!(
                "Failed to parse response for pipeline ID {}",
                pipeline_id
            )
        })?;

        Ok(definition.variables)
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
    fn test_client_creation_with_pat() {
        let result = AzureDevOpsClient::new(
            "https://dev.azure.com/myorg".to_string(),
            "myproject".to_string(),
            Some("test-pat-token".to_string()),
        );

        assert!(result.is_ok());
        let client = result.unwrap();
        assert_eq!(client.organization, "https://dev.azure.com/myorg");
        assert_eq!(client.project, "myproject");
    }

    #[test]
    fn test_client_creation_with_org_name() {
        let result = AzureDevOpsClient::new(
            "myorg".to_string(),
            "myproject".to_string(),
            Some("test-pat-token".to_string()),
        );

        assert!(result.is_ok());
        let client = result.unwrap();
        // Organization name should be normalized to full URL
        assert_eq!(client.organization, "https://dev.azure.com/myorg");
        assert_eq!(client.project, "myproject");
    }

    #[test]
    fn test_client_creation_preserves_full_url() {
        let result = AzureDevOpsClient::new(
            "https://dev.azure.com/customorg".to_string(),
            "myproject".to_string(),
            Some("test-pat-token".to_string()),
        );

        assert!(result.is_ok());
        let client = result.unwrap();
        // Full URL should be preserved as-is
        assert_eq!(client.organization, "https://dev.azure.com/customorg");
        assert_eq!(client.project, "myproject");
    }

    #[test]
    fn test_client_creation_without_pat_and_no_env() {
        // Temporarily unset AZDO_PAT if it exists
        let original = std::env::var("AZDO_PAT").ok();
        std::env::remove_var("AZDO_PAT");

        let result = AzureDevOpsClient::new(
            "myorg".to_string(),
            "myproject".to_string(),
            None,
        );

        // Restore original value
        if let Some(val) = original {
            std::env::set_var("AZDO_PAT", val);
        }

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("No authentication token provided"));
    }

    #[test]
    fn test_project_url_construction() {
        let client = AzureDevOpsClient::new(
            "myorg".to_string(),
            "myproject".to_string(),
            Some("test-pat".to_string()),
        )
        .unwrap();

        assert_eq!(
            client.project_url(),
            "https://dev.azure.com/myorg/myproject"
        );
    }

    #[test]
    fn test_parse_variable_group_response() {
        // Sample Azure DevOps REST API response for variable group
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

    // Tests for REST API response wrappers

    #[test]
    fn test_parse_variable_groups_response() {
        let json_response = r#"{
            "count": 1,
            "value": [{
                "id": 123,
                "name": "ProductionSecrets",
                "variables": {
                    "ApiKey": {"value": null, "isSecret": true}
                }
            }]
        }"#;

        let response: VariableGroupsResponse =
            serde_json::from_str(json_response).expect("Failed to parse");

        assert_eq!(response.value.len(), 1);
        assert_eq!(response.value[0].name, "ProductionSecrets");
    }

    #[test]
    fn test_parse_pipelines_response() {
        let json_response = r#"{
            "count": 2,
            "value": [
                {"id": 1, "name": "pipeline-1"},
                {"id": 2, "name": "pipeline-2"}
            ]
        }"#;

        let response: PipelinesResponse =
            serde_json::from_str(json_response).expect("Failed to parse");

        assert_eq!(response.value.len(), 2);
        assert_eq!(response.value[0].name, "pipeline-1");
        assert_eq!(response.value[1].name, "pipeline-2");
    }

    #[test]
    fn test_parse_build_definition_response() {
        let json_response = r#"{
            "id": 42,
            "name": "my-pipeline",
            "variables": {
                "BuildConfig": {
                    "value": "Release",
                    "isSecret": false,
                    "allowOverride": true
                }
            }
        }"#;

        let response: BuildDefinitionResponse =
            serde_json::from_str(json_response).expect("Failed to parse");

        assert!(response.variables.contains_key("BuildConfig"));
        let var = response.variables.get("BuildConfig").unwrap();
        assert_eq!(var.value, Some("Release".to_string()));
    }

    #[test]
    fn test_parse_empty_build_definition_response() {
        // Build definition with no variables
        let json_response = r#"{
            "id": 42,
            "name": "my-pipeline"
        }"#;

        let response: BuildDefinitionResponse =
            serde_json::from_str(json_response).expect("Failed to parse");

        assert!(response.variables.is_empty());
    }
}
