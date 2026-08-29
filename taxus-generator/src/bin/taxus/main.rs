// generator/src/bin/taxus/main.rs

//! Taxus - A static site generator with Tera and Yew.
//!
//! This is the CLI binary for the static site generator.
//! It uses the generator library to build static sites from Markdown content.

use crate::commands::{BuildArgs, InitArgs, ServeArgs};
use clap::Parser;
use taxus_lib::telemetry::{init, init_tracing};
use tracing::info;

mod cli;
mod commands;
mod error;

use cli::{Cli, Commands};
use commands::{run_build, run_clean, run_init, run_routes, run_serve};
use error::render_error;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // Initialize tracing based on command and flags
    match &cli.command {
        Commands::Build { verbose, quiet, .. } => {
            init_tracing(*verbose, *quiet);
        }
        Commands::Serve { verbose, quiet, .. } => {
            init_tracing(*verbose, *quiet);
        }
        _ => {
            init();
        }
    }

    // actions based on the CLI command
    match cli.command {
        Commands::Build {
            dir,
            verbose,
            quiet,
            include_drafts,
            dry_run,
            clean,
            output,
        } => {
            match run_build(&BuildArgs {
                dir,
                verbose,
                include_drafts,
                dry_run,
                clean,
                output,
            }) {
                Ok(report) => {
                    if !quiet {
                        report.print_summary();
                    }

                    if report.is_failure() {
                        std::process::exit(1);
                    }
                }
                Err(e) => {
                    render_error(&e);
                    std::process::exit(1);
                }
            }
        }
        Commands::Clean { dir } => match run_clean(&dir) {
            Ok(()) => {
                info!("✓ Output directory cleaned.");
            }
            Err(e) => {
                render_error(&e);
                std::process::exit(1);
            }
        },
        Commands::Init {
            path,
            name,
            base_url,
            force,
            no_islands,
        } => {
            match run_init(&InitArgs {
                path,
                name,
                base_url,
                force,
                islands: !no_islands,
            }) {
                Ok(report) => {
                    report.print_summary();
                }
                Err(e) => {
                    render_error(&e);
                    std::process::exit(1);
                }
            }
        }
        Commands::Routes { dir } => match run_routes(&dir) {
            Ok(()) => {}
            Err(e) => {
                render_error(&e);
                std::process::exit(1);
            }
        },
        Commands::Serve {
            dir,
            port,
            verbose: _,
            quiet,
            open,
        } => {
            match run_serve(&ServeArgs {
                dir,
                port,
                quiet,
                open,
            })
            .await
            {
                Ok(()) => {}
                Err(e) => {
                    render_error(&e);
                    std::process::exit(1);
                }
            }
        }
    }
}
