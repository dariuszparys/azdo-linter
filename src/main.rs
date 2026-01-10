use clap::Parser;
use std::process;

use azdo_linter::azure::AzureDevOpsClient;
use azdo_linter::error::OutputFormatter;
use azdo_linter::parser::{
    detect_template, extract_template_references, extract_variable_references,
    extract_variable_references_from_content, parse_pipeline_file, resolve_template_path,
};
use azdo_linter::validator::{validate_variable_groups, validate_variables, VariableSource};

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
            eprintln!("Error: {e}");
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

    // Check if this is a template file
    let template_info = detect_template(&args.pipeline_file)?;
    if template_info.is_template {
        println!(
            "{}",
            OutputFormatter::warning("This appears to be a template file")
        );
        println!();
        println!("  Template files cannot be validated in isolation because they expect");
        println!("  variables to be provided by the parent pipeline that includes them.");
        println!();
        if !template_info.parameter_names.is_empty() {
            println!("  Template parameters defined:");
            for param in &template_info.parameter_names {
                println!("    - {param}");
            }
            println!();
        }
        println!("  To validate variables used in this template, run the linter against");
        println!("  the parent pipeline that includes this template.");
        println!();
        println!("================================");
        println!("RESULT: SKIPPED (template file)");
        println!("================================");
        return Ok(false); // Exit successfully, not a validation failure
    }

    let pipeline = parse_pipeline_file(&args.pipeline_file)?;

    // Extract variable groups from the pipeline (searches all levels: top, stage, job)
    let variable_groups = pipeline.get_variable_groups();
    if args.verbose {
        println!("{}", OutputFormatter::info(&format!("Found {} variable group(s) referenced", variable_groups.len())));
        for group in &variable_groups {
            println!("       - {group}");
        }
    }

    // Extract inline variables defined in the pipeline
    let inline_variables = pipeline.get_inline_variable_names();
    if args.verbose {
        println!("{}", OutputFormatter::info(&format!("Found {} inline variable(s) defined", inline_variables.len())));
        for var in &inline_variables {
            println!("       - {var}");
        }
    }

    // Extract variable references from the pipeline
    // (excludes PowerShell expressions, system variables, and runtime outputs)
    let variable_references = extract_variable_references(&args.pipeline_file)?;
    if args.verbose {
        println!(
            "{}",
            OutputFormatter::info(&format!("Found {} variable reference(s) to validate", variable_references.len()))
        );
        for var in &variable_references {
            println!("       - $({var})");
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
                    println!("         Error: {error}");
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

    // Validate variables exist in groups or are defined inline
    let variable_results = validate_variables(variable_references, &group_results, &inline_variables, &client)?;

    // Track counts for summary
    let mut var_pass_count = 0;
    let mut var_fail_count = 0;

    // Print variable validation results
    for result in &variable_results {
        if result.exists {
            var_pass_count += 1;
            match &result.source {
                VariableSource::Group(group_name) => {
                    println!(
                        "{}",
                        OutputFormatter::success(&format!("Variable '{}' found in group '{}'", result.variable_name, group_name))
                    );
                }
                VariableSource::Inline => {
                    println!(
                        "{}",
                        OutputFormatter::success(&format!("Variable '{}' defined inline in pipeline", result.variable_name))
                    );
                }
                VariableSource::NotFound => {
                    // This shouldn't happen if exists is true, but handle it gracefully
                    println!("{}", OutputFormatter::success(&format!("Variable '{}' found", result.variable_name)));
                }
            }
        } else {
            var_fail_count += 1;
            println!(
                "{}",
                OutputFormatter::failure(&format!("Variable '{}' not found in any referenced group", result.variable_name))
            );
            if let Some(ref error) = result.error {
                if args.verbose {
                    println!("         Error: {error}");
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

    // Validate templates referenced in the pipeline
    let template_refs = extract_template_references(&args.pipeline_file)?;
    let mut template_pass_count = 0;
    let mut template_fail_count = 0;

    if !template_refs.is_empty() {
        for template_ref in &template_refs {
            let resolved_path = resolve_template_path(&args.pipeline_file, &template_ref.template_path);

            // Build section header
            let stage_info = template_ref
                .stage_name
                .as_ref()
                .map(|s| format!(" (stage: {s})"))
                .unwrap_or_default();
            let groups_info = if template_ref.available_groups.is_empty() {
                String::new()
            } else {
                format!(", groups: {}", template_ref.available_groups.join(", "))
            };

            println!(
                "{}",
                OutputFormatter::section(&format!(
                    "Template: {}{}{}",
                    template_ref.template_path, stage_info, groups_info
                ))
            );

            // Check if template file exists
            if !std::path::Path::new(&resolved_path).exists() {
                println!(
                    "{}",
                    OutputFormatter::warning(&format!(
                        "Template file not found: {} (resolved to: {})",
                        template_ref.template_path, resolved_path
                    ))
                );
                println!("         The template may be in a different repository or location.");
                continue;
            }

            // Read and extract variable references from template
            let template_content = std::fs::read_to_string(&resolved_path)?;
            let template_var_refs = extract_variable_references_from_content(&template_content)?;

            if template_var_refs.is_empty() {
                println!(
                    "{}",
                    OutputFormatter::info("No variable references found in template")
                );
                continue;
            }

            if args.verbose {
                println!(
                    "{}",
                    OutputFormatter::info(&format!(
                        "Found {} variable reference(s) in template",
                        template_var_refs.len()
                    ))
                );
            }

            // Validate template's variable groups exist (filter to only those we haven't validated yet)
            let new_groups: Vec<String> = template_ref
                .available_groups
                .iter()
                .filter(|g| !group_results.iter().any(|r| &r.group_name == *g))
                .cloned()
                .collect();

            let template_group_results = if !new_groups.is_empty() {
                validate_variable_groups(new_groups, &client)?
            } else {
                Vec::new()
            };

            // Combine all group results for validation
            let all_group_results: Vec<_> = group_results
                .iter()
                .chain(template_group_results.iter())
                .filter(|r| template_ref.available_groups.contains(&r.group_name))
                .cloned()
                .collect();

            // Validate template variables
            let template_var_results = validate_variables(
                template_var_refs,
                &all_group_results,
                &template_ref.available_inline_vars,
                &client,
            )?;

            // Print template variable validation results
            for result in &template_var_results {
                if result.exists {
                    template_pass_count += 1;
                    match &result.source {
                        VariableSource::Group(group_name) => {
                            println!(
                                "{}",
                                OutputFormatter::success(&format!(
                                    "Variable '{}' found in group '{}'",
                                    result.variable_name, group_name
                                ))
                            );
                        }
                        VariableSource::Inline => {
                            println!(
                                "{}",
                                OutputFormatter::success(&format!(
                                    "Variable '{}' defined inline in parent pipeline",
                                    result.variable_name
                                ))
                            );
                        }
                        VariableSource::NotFound => {
                            println!(
                                "{}",
                                OutputFormatter::success(&format!("Variable '{}' found", result.variable_name))
                            );
                        }
                    }
                } else {
                    template_fail_count += 1;
                    println!(
                        "{}",
                        OutputFormatter::failure(&format!(
                            "Variable '{}' not found in available groups",
                            result.variable_name
                        ))
                    );
                    if !template_ref.available_groups.is_empty() {
                        println!(
                            "         Available groups: {}",
                            template_ref.available_groups.join(", ")
                        );
                    }
                    println!("         Suggestion: Add this variable to one of the available variable groups.");
                }
            }
        }
    }

    // Calculate totals
    let total_passed = group_pass_count + var_pass_count + template_pass_count;
    let total_failed = group_fail_count + var_fail_count + template_fail_count;

    // Print summary using OutputFormatter
    println!("{}", OutputFormatter::summary(total_passed, total_failed));

    Ok(total_failed > 0)
}
