//! Yew SSG - A Yew-based Static Site Generator
//!
//! This is the CLI binary for the static site generator.
//! It uses the generator library to build static sites from Markdown content.

use clap::{Parser, Subcommand};
use generator::{BuildReport, InitOptions, InitReport, InitScaffolder, SiteBuilder};
use std::io::{self, BufRead, Write};
use std::path::PathBuf;

/// A Yew-based static site generator
#[derive(Parser)]
#[command(name = "yew-ssg")]
#[command(about = "A Yew-based static site generator", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Build the static site
    Build {
        /// Directory containing site.toml
        #[arg(short, long, default_value = ".")]
        dir: PathBuf,

        /// Enable verbose output
        #[arg(short, long)]
        verbose: bool,

        /// Include draft pages in build
        #[arg(long)]
        include_drafts: bool,

        /// Dry run - don't write files
        #[arg(long)]
        dry_run: bool,

        /// Clean output directory before build
        #[arg(long)]
        clean: bool,
    },

    /// Initialize a new site
    Init {
        /// Directory to initialize (defaults to current directory)
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Site name
        #[arg(short, long)]
        name: Option<String>,

        /// Base URL for the site
        #[arg(short = 'u', long)]
        base_url: Option<String>,

        /// Initialize even if directory is not empty
        #[arg(short, long)]
        force: bool,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Build {
            dir,
            verbose,
            include_drafts,
            dry_run,
            clean,
        } => {
            match run_build(&BuildArgs {
                dir,
                verbose,
                include_drafts,
                dry_run,
                clean,
            }) {
                Ok(report) => {
                    report.print_summary();

                    if report.has_warnings() {
                        std::process::exit(1);
                    }
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }
        Commands::Init {
            path,
            name,
            base_url,
            force,
        } => {
            match run_init(&InitArgs {
                path,
                name,
                base_url,
                force,
            }) {
                Ok(report) => {
                    report.print_summary();
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }
    }
}

struct BuildArgs {
    dir: PathBuf,
    verbose: bool,
    include_drafts: bool,
    dry_run: bool,
    clean: bool,
}

fn run_build(args: &BuildArgs) -> Result<BuildReport, generator::error::GeneratorError> {
    // Clean if requested
    if args.clean {
        if args.verbose {
            println!("Cleaning output directory...");
        }
        SiteBuilder::from_dir(&args.dir)?.clean()?;
    }

    // Build the site
    let report = SiteBuilder::from_dir(&args.dir)?
        .verbose(args.verbose)
        .dry_run(args.dry_run)
        .include_drafts(args.include_drafts)
        .build()?;

    Ok(report)
}

struct InitArgs {
    path: PathBuf,
    name: Option<String>,
    base_url: Option<String>,
    force: bool,
}

fn run_init(args: &InitArgs) -> Result<InitReport, generator::error::GeneratorError> {
    use generator::init::{derive_site_name, is_directory_empty};

    // Check if directory is empty
    if !args.force {
        let is_empty = is_directory_empty(&args.path)?;
        if !is_empty {
            // Prompt user for confirmation
            print!("Directory '{}' is not empty. Continue? (y/N): ", args.path.display());
            io::stdout().flush().ok();

            let stdin = io::stdin();
            let mut input = String::new();
            if stdin.lock().read_line(&mut input).is_ok() {
                let trimmed = input.trim().to_lowercase();
                if trimmed != "y" && trimmed != "yes" {
                    return Err(generator::error::InitError::Cancelled.into());
                }
            } else {
                return Err(generator::error::InitError::Cancelled.into());
            }
        }
    }

    // Derive site name from path if not provided
    let name = args.name.clone().unwrap_or_else(|| derive_site_name(&args.path));

    // Use default base URL if not provided
    let base_url = args.base_url.clone().unwrap_or_else(|| "https://example.com".to_string());

    // Create options and scaffolder
    let options = InitOptions::new(&name, &base_url).with_force(args.force);
    let scaffolder = InitScaffolder::new(options);

    // Scaffold the site
    let report = scaffolder.scaffold(&args.path)?;

    Ok(report)
}
