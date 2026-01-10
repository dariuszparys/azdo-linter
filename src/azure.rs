//! Azure CLI integration for Azure DevOps API calls

use anyhow::{Context, Result};
use std::process::Command;

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
}
