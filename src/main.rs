use clap::Parser;

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

fn main() {
    let args = Args::parse();

    if args.verbose {
        println!("Pipeline file: {}", args.pipeline_file);
        println!("Organization: {}", args.organization);
        println!("Project: {}", args.project);
    }

    println!("Azure DevOps Pipeline Validator");
}
