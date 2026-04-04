// generator/src/bin/taxus/commands.rs

use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use taxus_lib::error::{GeneratorError, InitError};
use taxus_lib::{BuildReport, InitOptions, InitReport, InitScaffolder, SiteBuilder};

// ---------------------------------------------------------------------------
// Serve
// ---------------------------------------------------------------------------

pub struct ServeArgs {
    pub dir: PathBuf,
    pub port: u16,
    pub quiet: bool,
    pub open: bool,
}

pub async fn run_serve(args: &ServeArgs) -> Result<(), Box<GeneratorError>> {
    use taxus_lib::serve::{DevServer, DevServerConfig, RebuildFn};

    // Load config to get output directory
    let config = taxus_lib::SiteConfig::from_dir(&args.dir)?;

    // Create server configuration
    let server_config = DevServerConfig::default()
        .with_port(args.port)
        .with_output_dir(config.build.output_dir.clone())
        .with_site_dir(args.dir.clone());

    // Capture what the rebuild needs in the closure
    let site_dir = args.dir.clone();
    let include_drafts = false;

    let rebuild: RebuildFn = Arc::new(move || {
        taxus_lib::SiteBuilder::from_dir(&site_dir)
            .map_err(|e| e.to_string())?
            .include_drafts(include_drafts)
            .build()
            .map_err(|e| e.to_string())?;
        Ok(())
    });

    if !args.quiet {
        tracing::info!("Starting development server...");
        tracing::info!("Site: {}", args.dir.display());
        tracing::info!("Output: {}", config.build.output_dir.display());
        tracing::info!("Port: {}", args.port);
    }

    // Create and run the server
    let server = DevServer::new(server_config, rebuild);

    if !args.quiet {
        tracing::info!("\n  Static site: http://localhost:{}", args.port);
        tracing::info!("  Press Ctrl+C to stop\n");
    }

    // Open browser if requested
    if args.open {
        let url = format!("http://localhost:{}", args.port);
        if let Err(e) = webbrowser::open(&url) {
            tracing::error!("Warning: Failed to open browser: {}", e);
        }
    }

    server.run().await.map_err(|e| {
        GeneratorError::Serve(taxus_lib::serve::ServeError::Server(format!(
            "Server error: {}",
            e
        )))
    })?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Build
// ---------------------------------------------------------------------------

pub struct BuildArgs {
    pub dir: PathBuf,
    pub verbose: bool,
    pub include_drafts: bool,
    pub dry_run: bool,
    pub clean: bool,
    pub output: Option<PathBuf>,
}

pub fn run_build(args: &BuildArgs) -> Result<BuildReport, Box<GeneratorError>> {
    // Load config so we can apply the --output override before building
    let mut config = taxus_lib::SiteConfig::from_dir(&args.dir)?;
    if let Some(ref output) = args.output {
        config.build.output_dir = output.clone();
    }

    // Clean if requested — create a temporary builder just for the clean step
    if args.clean {
        if args.verbose {
            println!("Cleaning output directory...");
        }
        SiteBuilder::new(config.clone()).clean()?;
    }

    // Build the site
    let report = SiteBuilder::new(config)
        .verbose(args.verbose)
        .dry_run(args.dry_run)
        .include_drafts(args.include_drafts)
        .build()?;

    Ok(report)
}

// ---------------------------------------------------------------------------
// Clean
// ---------------------------------------------------------------------------

pub fn run_clean(dir: &Path) -> Result<(), Box<GeneratorError>> {
    Ok(SiteBuilder::from_dir(dir)?.clean()?)
}

// ---------------------------------------------------------------------------
// Init
// ---------------------------------------------------------------------------

pub struct InitArgs {
    pub path: PathBuf,
    pub name: Option<String>,
    pub base_url: Option<String>,
    pub force: bool,
    pub islands: bool,
}

pub fn run_init(args: &InitArgs) -> Result<InitReport, Box<GeneratorError>> {
    use taxus_lib::init::{derive_site_name, is_directory_empty};

    // Check if directory is empty
    if !args.force {
        let is_empty = is_directory_empty(&args.path)?;
        if !is_empty {
            // Prompt user for confirmation
            print!(
                "Directory '{}' is not empty. Continue? (y/N): ",
                args.path.display()
            );
            io::stdout().flush().ok();

            let stdin = io::stdin();
            let mut input = String::new();
            if stdin.lock().read_line(&mut input).is_ok() {
                let trimmed = input.trim().to_lowercase();
                if trimmed != "y" && trimmed != "yes" {
                    return Err(InitError::Cancelled.into());
                }
            } else {
                return Err(InitError::Cancelled.into());
            }
        }
    }

    // Derive site name from path if not provided
    let name = args
        .name
        .clone()
        .unwrap_or_else(|| derive_site_name(&args.path));

    // Use default base URL if not provided
    let base_url = args
        .base_url
        .clone()
        .unwrap_or_else(|| "https://example.com".to_string());

    // Create options and scaffolder
    let options = InitOptions::new(&name, &base_url)
        .with_force(args.force)
        .with_islands(args.islands);
    let scaffolder = InitScaffolder::new(options);

    // Scaffold the site
    let report = scaffolder.scaffold(&args.path)?;

    Ok(report)
}

// ---------------------------------------------------------------------------
// Routes
// ---------------------------------------------------------------------------

pub fn run_routes(dir: &Path) -> Result<(), Box<GeneratorError>> {
    use taxus_lib::{RouteDiscovery, SiteConfig};

    let config = SiteConfig::from_dir(dir)?;
    let discovery = RouteDiscovery::new(&config.build.content_dir);
    let registry = discovery.discover()?;

    tracing::info!(
        "\nRoutes for \"{}\"\n─────────────────────────────────────────────────────",
        config.site.name
    );

    // Collect and sort routes for a stable display order
    let mut routes: Vec<_> = registry.iter().collect();
    routes.sort_by(|a, b| a.path.cmp(&b.path));

    for route in &routes {
        let kind = if route.is_section() {
            "section"
        } else {
            "page"
        };
        println!(
            "  [{kind:<7}]  {:<28}  {:<30}  {}",
            route.path,
            route.content_file.display(),
            route.output_file.display()
        );
    }

    let page_count = registry.pages().count();
    let section_count = registry.sections().count();
    println!("─────────────────────────────────────────────────────");
    println!(
        "  Total: {} route{} ({} page{}, {} section{})\n",
        routes.len(),
        if routes.len() == 1 { "" } else { "s" },
        page_count,
        if page_count == 1 { "" } else { "s" },
        section_count,
        if section_count == 1 { "" } else { "s" }
    );

    Ok(())
}
