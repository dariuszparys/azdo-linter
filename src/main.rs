use clap::Parser;
use std::process;

use azdo_linter::azure::AzureDevOpsClient;
use azdo_linter::parser::{extract_variable_references, parse_pipeline_file};
use azdo_linter::validator::{validate_variable_groups, validate_variables};

/// Azure DevOps pipeline YAML validator
///
/// Validates that variable groups and variables referenced in Azure DevOps
/// pipeline YAML files actually exist in Azure DevOps.
#[derive(Parser, Debug)]
#[command(name = "azdo-linter")]
#[command(about = "Validates Azure DevOps pipeline YAML variable references")]
struct Args {
    /// Path to the Azure DevOps pipeline YAML file to validate
    #[arg(short, long)]
    pipeline_file: String,

    /// Azure DevOps organization name (e.g., 'myorg' from https://dev.azure.com/myorg)
    #[arg(short, long)]
    organization: String,

    /// Azure DevOps project name
    #[arg(short = 'j', long)]
    project: String,

    /// Enable verbose output for debugging
    #[arg(short, long, default_value_t = false)]
    verbose: bool,
}

/// Exit codes for the validator
/// 0 = Success (all validations passed)
/// 1 = Validation failures (some variable groups or variables not found)
/// 2 = Error (could not complete validation due to errors)
const EXIT_SUCCESS: i32 = 0;
const EXIT_VALIDATION_FAILURE: i32 = 1;
const EXIT_ERROR: i32 = 2;

fn main() {
    let args = Args::parse();

    if args.verbose {
        println!("Pipeline file: {}", args.pipeline_file);
        println!("Organization: {}", args.organization);
        println!("Project: {}", args.project);
    }

    match run_validation(&args) {
        Ok(has_failures) => {
            if has_failures {
                process::exit(EXIT_VALIDATION_FAILURE);
            } else {
                process::exit(EXIT_SUCCESS);
            }
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(EXIT_ERROR);
        }
    }
}

/// Run the validation workflow and return whether any validation failures occurred
fn run_validation(args: &Args) -> Result<bool, anyhow::Error> {
    println!("Azure DevOps Pipeline Validator");
    println!("================================");
    println!();

    // Parse the pipeline file
    if args.verbose {
        println!("Parsing pipeline file: {}", args.pipeline_file);
    }
    let pipeline = parse_pipeline_file(&args.pipeline_file)?;

    // Extract variable groups from the pipeline
    let variable_groups = pipeline.get_variable_groups();
    if args.verbose {
        println!("Found {} variable group(s) referenced", variable_groups.len());
        for group in &variable_groups {
            println!("  - {}", group);
        }
    }

    // Extract variable references from the pipeline
    let variable_references = extract_variable_references(&args.pipeline_file)?;
    if args.verbose {
        println!(
            "Found {} variable reference(s) in pipeline",
            variable_references.len()
        );
        for var in &variable_references {
            println!("  - $({})", var);
        }
    }

    // Initialize Azure DevOps client
    let client = AzureDevOpsClient::new(args.organization.clone(), args.project.clone());

    // Check Azure CLI availability
    if args.verbose {
        println!("Checking Azure CLI availability...");
    }
    client.check_cli_available()?;
    if args.verbose {
        println!("Azure CLI is available");
    }

    println!();
    println!("Validating variable groups...");

    // Validate variable groups exist
    let group_results = validate_variable_groups(variable_groups, &client)?;

    // Print group validation results
    let mut has_group_failures = false;
    for result in &group_results {
        if result.exists {
            println!("  [PASS] Variable group '{}' exists", result.group_name);
        } else {
            has_group_failures = true;
            println!("  [FAIL] Variable group '{}' not found", result.group_name);
            if let Some(ref error) = result.error {
                if args.verbose {
                    println!("         Error: {}", error);
                }
            }
        }
    }

    if group_results.is_empty() {
        println!("  No variable groups referenced in pipeline");
    }

    println!();
    println!("Validating variable references...");

    // Validate variables exist in groups
    let variable_results = validate_variables(variable_references, &group_results, &client)?;

    // Print variable validation results
    let mut has_variable_failures = false;
    for result in &variable_results {
        if result.exists {
            if let Some(ref group_name) = result.group_name {
                println!(
                    "  [PASS] Variable '{}' found in group '{}'",
                    result.variable_name, group_name
                );
            } else {
                println!("  [PASS] Variable '{}' found", result.variable_name);
            }
        } else {
            has_variable_failures = true;
            println!(
                "  [FAIL] Variable '{}' not found in any referenced group",
                result.variable_name
            );
            if let Some(ref error) = result.error {
                if args.verbose {
                    println!("         Error: {}", error);
                }
            }
        }
    }

    if variable_results.is_empty() {
        println!("  No variable references found in pipeline");
    }

    // Print summary
    println!();
    println!("================================");
    let total_failures =
        group_results.iter().filter(|r| !r.exists).count()
            + variable_results.iter().filter(|r| !r.exists).count();

    if total_failures == 0 {
        println!("Validation PASSED: All variable groups and variables exist");
    } else {
        println!(
            "Validation FAILED: {} issue(s) found",
            total_failures
        );
    }

    Ok(has_group_failures || has_variable_failures)
}
