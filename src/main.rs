use clap::Parser;
use std::process;

use azdo_linter::azure::AzureDevOpsClient;
use azdo_linter::error::OutputFormatter;
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
        println!("{}", OutputFormatter::info(&format!("Parsing pipeline file: {}", args.pipeline_file)));
    }
    let pipeline = parse_pipeline_file(&args.pipeline_file)?;

    // Extract variable groups from the pipeline
    let variable_groups = pipeline.get_variable_groups();
    if args.verbose {
        println!("{}", OutputFormatter::info(&format!("Found {} variable group(s) referenced", variable_groups.len())));
        for group in &variable_groups {
            println!("       - {}", group);
        }
    }

    // Extract variable references from the pipeline
    let variable_references = extract_variable_references(&args.pipeline_file)?;
    if args.verbose {
        println!(
            "{}",
            OutputFormatter::info(&format!("Found {} variable reference(s) in pipeline", variable_references.len()))
        );
        for var in &variable_references {
            println!("       - $({})", var);
        }
    }

    // Initialize Azure DevOps client
    let client = AzureDevOpsClient::new(args.organization.clone(), args.project.clone());

    // Check Azure CLI availability
    if args.verbose {
        println!("{}", OutputFormatter::info("Checking Azure CLI availability..."));
    }
    client.check_cli_available()?;
    if args.verbose {
        println!("{}", OutputFormatter::success("Azure CLI is available and configured"));
    }

    println!("{}", OutputFormatter::section("Variable Groups"));

    // Validate variable groups exist
    let group_results = validate_variable_groups(variable_groups, &client)?;

    // Track counts for summary
    let mut group_pass_count = 0;
    let mut group_fail_count = 0;

    // Print group validation results
    for result in &group_results {
        if result.exists {
            group_pass_count += 1;
            println!("{}", OutputFormatter::success(&format!("Variable group '{}' exists", result.group_name)));
        } else {
            group_fail_count += 1;
            println!("{}", OutputFormatter::failure(&format!("Variable group '{}' not found", result.group_name)));
            if let Some(ref error) = result.error {
                if args.verbose {
                    println!("         Error: {}", error);
                }
            }
            // Provide actionable suggestion
            println!(
                "         Suggestion: Create the variable group in Azure DevOps at:\n         https://dev.azure.com/{}/{}/_library?itemType=VariableGroups",
                args.organization, args.project
            );
        }
    }

    if group_results.is_empty() {
        println!("{}", OutputFormatter::info("No variable groups referenced in pipeline"));
    }

    println!("{}", OutputFormatter::section("Variable References"));

    // Validate variables exist in groups
    let variable_results = validate_variables(variable_references, &group_results, &client)?;

    // Track counts for summary
    let mut var_pass_count = 0;
    let mut var_fail_count = 0;

    // Print variable validation results
    for result in &variable_results {
        if result.exists {
            var_pass_count += 1;
            if let Some(ref group_name) = result.group_name {
                println!(
                    "{}",
                    OutputFormatter::success(&format!("Variable '{}' found in group '{}'", result.variable_name, group_name))
                );
            } else {
                println!("{}", OutputFormatter::success(&format!("Variable '{}' found", result.variable_name)));
            }
        } else {
            var_fail_count += 1;
            println!(
                "{}",
                OutputFormatter::failure(&format!("Variable '{}' not found in any referenced group", result.variable_name))
            );
            if let Some(ref error) = result.error {
                if args.verbose {
                    println!("         Error: {}", error);
                }
            }
            // Provide actionable suggestion
            println!("         Suggestion: Add this variable to one of the referenced variable groups,");
            println!("         or verify the variable name is spelled correctly.");
        }
    }

    if variable_results.is_empty() {
        println!("{}", OutputFormatter::info("No variable references found in pipeline"));
    }

    // Calculate totals
    let total_passed = group_pass_count + var_pass_count;
    let total_failed = group_fail_count + var_fail_count;

    // Print summary using OutputFormatter
    println!("{}", OutputFormatter::summary(total_passed, total_failed));

    Ok(total_failed > 0)
}
