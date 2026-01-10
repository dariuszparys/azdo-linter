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

/// Top-level pipeline structure
#[derive(Debug, Deserialize)]
pub struct Pipeline {
    /// Variables section containing both inline variables and group references
    #[serde(default)]
    pub variables: Option<Vec<VariableEntry>>,
}

impl Pipeline {
    /// Extract all variable group names referenced in the pipeline
    ///
    /// # Returns
    /// * `Vec<String>` - Unique list of variable group names
    pub fn get_variable_groups(&self) -> Vec<String> {
        let mut groups = Vec::new();

        if let Some(ref variables) = self.variables {
            for entry in variables {
                if let VariableEntry::Group { group } = entry {
                    if !groups.contains(group) {
                        groups.push(group.clone());
                    }
                }
            }
        }

        groups
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

/// Extract variable references from raw YAML content string
///
/// # Arguments
/// * `content` - Raw YAML content
///
/// # Returns
/// * `Result<Vec<String>>` - Unique list of variable names referenced
pub fn extract_variable_references_from_content(content: &str) -> Result<Vec<String>> {
    // Regex pattern to match $(variableName) syntax
    // Captures the variable name inside the parentheses
    let re = Regex::new(r"\$\(([^)]+)\)")
        .with_context(|| "Failed to compile variable reference regex")?;

    let mut variables = Vec::new();

    for cap in re.captures_iter(content) {
        if let Some(var_name) = cap.get(1) {
            let name = var_name.as_str().to_string();
            if !variables.contains(&name) {
                variables.push(name);
            }
        }
    }

    Ok(variables)
}
