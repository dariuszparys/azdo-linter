//! YAML parser for Azure DevOps pipeline files

use anyhow::{Context, Result};
use regex::Regex;
use serde::Deserialize;
use std::fs;

/// Represents a variable group reference in the pipeline
#[derive(Debug, Deserialize)]
pub struct VariableGroup {
    /// Name of the variable group
    pub group: Option<String>,
    /// Individual variables (when not a group reference)
    #[serde(flatten)]
    pub variables: Option<std::collections::HashMap<String, String>>,
}

/// Represents an individual variable definition
#[derive(Debug, Deserialize)]
pub struct Variable {
    /// Variable name
    pub name: Option<String>,
    /// Variable value
    pub value: Option<String>,
}

/// Represents a variable entry in the variables section
/// Azure DevOps YAML supports multiple formats:
/// - group: 'GroupName' (variable group reference)
/// - name: 'VarName' + value: 'VarValue' (inline variable)
/// - template: 'path/to/template.yml' (template reference)
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum VariableEntry {
    /// Variable group reference: - group: 'GroupName'
    Group { group: String },
    /// Named variable: - name: 'VarName' value: 'VarValue'
    Named { name: String, value: Option<String> },
    /// Template reference: - template: 'path'
    Template { template: String },
}

/// Represents a job in a stage
#[derive(Debug, Deserialize)]
pub struct Job {
    /// Job-level variables
    #[serde(default)]
    pub variables: Option<Vec<VariableEntry>>,
}

/// Represents a deployment job in a stage
#[derive(Debug, Deserialize)]
pub struct Deployment {
    /// Deployment-level variables
    #[serde(default)]
    pub variables: Option<Vec<VariableEntry>>,
}

/// Represents a stage in the pipeline
#[derive(Debug, Deserialize)]
pub struct Stage {
    /// Stage name
    #[serde(default)]
    pub stage: Option<String>,
    /// Stage-level variables
    #[serde(default)]
    pub variables: Option<Vec<VariableEntry>>,
    /// Jobs in the stage
    #[serde(default)]
    pub jobs: Option<Vec<Job>>,
}

/// Top-level pipeline structure
#[derive(Debug, Deserialize)]
pub struct Pipeline {
    /// Variables section containing both inline variables and group references
    #[serde(default)]
    pub variables: Option<Vec<VariableEntry>>,
    /// Stages in the pipeline
    #[serde(default)]
    pub stages: Option<Vec<Stage>>,
}

impl Pipeline {
    /// Extract all variable group names referenced in the pipeline
    /// Searches top-level variables, stage-level variables, and job-level variables
    ///
    /// # Returns
    /// * `Vec<String>` - Unique list of variable group names
    pub fn get_variable_groups(&self) -> Vec<String> {
        let mut groups = Vec::new();

        // Collect from top-level variables
        Self::collect_groups_from_variables(&self.variables, &mut groups);

        // Collect from stage-level variables
        if let Some(ref stages) = self.stages {
            for stage in stages {
                Self::collect_groups_from_variables(&stage.variables, &mut groups);

                // Collect from job-level variables
                if let Some(ref jobs) = stage.jobs {
                    for job in jobs {
                        Self::collect_groups_from_variables(&job.variables, &mut groups);
                    }
                }
            }
        }

        groups
    }

    /// Helper function to collect variable groups from a variables section
    fn collect_groups_from_variables(
        variables: &Option<Vec<VariableEntry>>,
        groups: &mut Vec<String>,
    ) {
        if let Some(ref vars) = variables {
            for entry in vars {
                if let VariableEntry::Group { group } = entry {
                    if !groups.contains(group) {
                        groups.push(group.clone());
                    }
                }
            }
        }
    }

    /// Extract all inline variable names defined in the pipeline
    /// Searches top-level variables, stage-level variables, and job-level variables
    ///
    /// # Returns
    /// * `Vec<String>` - Unique list of inline variable names
    pub fn get_inline_variable_names(&self) -> Vec<String> {
        let mut names = Vec::new();

        // Collect from top-level variables
        Self::collect_inline_variables(&self.variables, &mut names);

        // Collect from stage-level variables
        if let Some(ref stages) = self.stages {
            for stage in stages {
                Self::collect_inline_variables(&stage.variables, &mut names);

                // Collect from job-level variables
                if let Some(ref jobs) = stage.jobs {
                    for job in jobs {
                        Self::collect_inline_variables(&job.variables, &mut names);
                    }
                }
            }
        }

        names
    }

    /// Helper function to collect inline variable names from a variables section
    fn collect_inline_variables(
        variables: &Option<Vec<VariableEntry>>,
        names: &mut Vec<String>,
    ) {
        if let Some(ref vars) = variables {
            for entry in vars {
                if let VariableEntry::Named { name, .. } = entry {
                    if !names.contains(name) {
                        names.push(name.clone());
                    }
                }
            }
        }
    }
}

/// Parse a pipeline YAML file and return the Pipeline structure
///
/// # Arguments
/// * `path` - Path to the YAML pipeline file
///
/// # Returns
/// * `Result<Pipeline>` - Parsed pipeline or error
pub fn parse_pipeline_file(path: &str) -> Result<Pipeline> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read pipeline file: {}", path))?;

    let pipeline: Pipeline = serde_yaml::from_str(&content)
        .with_context(|| format!("Failed to parse YAML in pipeline file: {}", path))?;

    Ok(pipeline)
}

/// Extract all variable references from pipeline YAML content
///
/// Finds all occurrences of $(variableName) syntax in the YAML content
/// and returns a unique list of variable names.
///
/// # Arguments
/// * `path` - Path to the YAML pipeline file
///
/// # Returns
/// * `Result<Vec<String>>` - Unique list of variable names referenced
pub fn extract_variable_references(path: &str) -> Result<Vec<String>> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read pipeline file: {}", path))?;

    extract_variable_references_from_content(&content)
}

/// Azure DevOps system variable prefixes that should be skipped during validation
const SYSTEM_VARIABLE_PREFIXES: &[&str] = &[
    "Build.",
    "System.",
    "Agent.",
    "Pipeline.",
    "Environment.",
    "Checks.",
    "Release.",
    "Task.",
    "Resources.",
];

/// Check if a variable name is a system/predefined Azure DevOps variable
pub fn is_system_variable(name: &str) -> bool {
    SYSTEM_VARIABLE_PREFIXES
        .iter()
        .any(|prefix| name.starts_with(prefix))
}

/// Check if a variable name is a runtime output variable
/// These are set dynamically during pipeline execution and cannot be validated statically
/// Examples: outputs.registryName, agentIp.value, domains.domainId
fn is_runtime_output_variable(name: &str) -> bool {
    // Must contain a dot to be a potential runtime output
    if !name.contains('.') {
        return false;
    }

    let parts: Vec<&str> = name.split('.').collect();
    if parts.len() < 2 {
        return false;
    }

    // Skip known system variable prefixes (handled separately)
    if is_system_variable(name) {
        return false;
    }

    // Anything else with a dot is likely a runtime output
    // e.g., outputs.registryName, agentIp.value, domains.domainId
    true
}

/// Check if a variable pattern should be skipped during validation
fn should_skip_variable(name: &str) -> bool {
    // Skip PowerShell expressions: $($outputs.foo), $($env:VAR)
    if name.starts_with('$') {
        return true;
    }

    // Skip template expressions: $[ ... ]
    if name.starts_with('[') {
        return true;
    }

    // Skip system variables
    if is_system_variable(name) {
        return true;
    }

    // Skip runtime output variables
    if is_runtime_output_variable(name) {
        return true;
    }

    false
}

/// Extract variable references from raw YAML content string
/// Filters out PowerShell expressions, system variables, and runtime output variables
///
/// # Arguments
/// * `content` - Raw YAML content
///
/// # Returns
/// * `Result<Vec<String>>` - Unique list of variable names referenced (excluding system/runtime vars)
pub fn extract_variable_references_from_content(content: &str) -> Result<Vec<String>> {
    // Regex pattern to match $(variableName) syntax
    // Captures the variable name inside the parentheses
    let re = Regex::new(r"\$\(([^)]+)\)")
        .with_context(|| "Failed to compile variable reference regex")?;

    let mut variables = Vec::new();

    for cap in re.captures_iter(content) {
        if let Some(var_name) = cap.get(1) {
            let name = var_name.as_str();

            // Skip variables that shouldn't be validated
            if should_skip_variable(name) {
                continue;
            }

            let name_string = name.to_string();
            if !variables.contains(&name_string) {
                variables.push(name_string);
            }
        }
    }

    Ok(variables)
}
