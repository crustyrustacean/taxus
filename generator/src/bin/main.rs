//! Yew SSG - A Yew-based Static Site Generator
//!
//! This is the CLI binary for the static site generator.
//! It uses the generator library to build static sites from Markdown content.

use clap::Parser;
use generator::{BuildReport, SiteBuilder};
use std::path::PathBuf;

/// A Yew-based static site generator
#[derive(Parser)]
#[command(name = "yew-ssg")]
#[command(about = "A Yew-based static site generator", long_about = None)]
#[command(version)]
struct Cli {
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
}

fn main() {
    let cli = Cli::parse();

    // Build the site
    match run_build(&cli) {
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

fn run_build(cli: &Cli) -> Result<BuildReport, generator::error::GeneratorError> {
    // Clean if requested
    if cli.clean {
        if cli.verbose {
            println!("Cleaning output directory...");
        }
        SiteBuilder::from_dir(&cli.dir)?.clean()?;
    }

    // Build the site
    let report = SiteBuilder::from_dir(&cli.dir)?
        .verbose(cli.verbose)
        .dry_run(cli.dry_run)
        .include_drafts(cli.include_drafts)
        .build()?;

    Ok(report)
}
