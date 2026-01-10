//! Error types for pipeline validation

use std::error::Error;
use std::fmt;

/// Error when parsing a pipeline YAML file fails
#[derive(Debug)]
pub struct PipelineParseError {
    /// Path to the file that failed to parse
    pub file_path: String,
    /// Underlying error message
    pub message: String,
}

impl fmt::Display for PipelineParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Failed to parse pipeline file '{}': {}\n\nSuggestion: Ensure the file exists and contains valid Azure DevOps YAML syntax.",
            self.file_path, self.message
        )
    }
}

impl Error for PipelineParseError {}

/// Error when Azure CLI is not available or not configured
#[derive(Debug)]
pub struct AzureCliError {
    /// The specific error encountered
    pub message: String,
}

impl fmt::Display for AzureCliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Azure CLI error: {}\n\nSuggestion: Ensure Azure CLI is installed and you are logged in with 'az login'. \
            Also verify the Azure DevOps extension is installed with 'az extension add --name azure-devops'.",
            self.message
        )
    }
}

impl Error for AzureCliError {}

/// Error when a variable group is not found in Azure DevOps
#[derive(Debug)]
pub struct VariableGroupNotFoundError {
    /// Name of the variable group that was not found
    pub group_name: String,
    /// Organization where it was searched
    pub organization: String,
    /// Project where it was searched
    pub project: String,
}

impl fmt::Display for VariableGroupNotFoundError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Variable group '{}' not found in organization '{}', project '{}'.\n\n\
            Suggestion: Verify the variable group name is correct and exists in Azure DevOps. \
            You can create it at: https://dev.azure.com/{}/{}/_library?itemType=VariableGroups",
            self.group_name, self.organization, self.project, self.organization, self.project
        )
    }
}

impl Error for VariableGroupNotFoundError {}

/// Error when a variable is not found in any referenced variable groups
#[derive(Debug)]
pub struct VariableNotFoundError {
    /// Name of the variable that was not found
    pub variable_name: String,
    /// List of groups that were searched
    pub searched_groups: Vec<String>,
}

impl fmt::Display for VariableNotFoundError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.searched_groups.is_empty() {
            write!(
                f,
                "Variable '{}' not found: No variable groups are referenced in the pipeline.\n\n\
                Suggestion: Add a variable group reference to your pipeline YAML, or define the variable inline.",
                self.variable_name
            )
        } else {
            write!(
                f,
                "Variable '{}' not found in any of the referenced variable groups: {}.\n\n\
                Suggestion: Add this variable to one of the referenced groups, or verify the variable name is spelled correctly.",
                self.variable_name,
                self.searched_groups.join(", ")
            )
        }
    }
}

impl Error for VariableNotFoundError {}

/// Error when validation encounters an unexpected issue
#[derive(Debug)]
pub struct ValidationError {
    /// Context about what was being validated
    pub context: String,
    /// The error message
    pub message: String,
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Validation error while {}: {}\n\nSuggestion: Check your Azure DevOps connection and permissions.",
            self.context, self.message
        )
    }
}

impl Error for ValidationError {}

/// Output formatting helpers for validation results
pub struct OutputFormatter;

impl OutputFormatter {
    /// Format a success indicator
    pub fn success(message: &str) -> String {
        format!("  [PASS] {message}")
    }

    /// Format a failure indicator
    pub fn failure(message: &str) -> String {
        format!("  [FAIL] {message}")
    }

    /// Format an info message
    pub fn info(message: &str) -> String {
        format!("  [INFO] {message}")
    }

    /// Format a section header
    pub fn section(title: &str) -> String {
        format!("\n{}\n{}", title, "-".repeat(title.len()))
    }

    /// Format the final summary
    pub fn summary(passed: usize, failed: usize) -> String {
        let total = passed + failed;
        if failed == 0 {
            format!(
                "\n================================\n\
                 RESULT: PASSED\n\
                 All {total} check(s) passed successfully.\n\
                 ================================"
            )
        } else {
            format!(
                "\n================================\n\
                 RESULT: FAILED\n\
                 {failed} of {total} check(s) failed.\n\
                 ================================"
            )
        }
    }
}
