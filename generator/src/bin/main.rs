//! Yew SSG - A Yew-based Static Site Generator
//!
//! This is the CLI binary for the static site generator.
//! It uses the generator library to build static sites from Markdown content.

use clap::{Parser, Subcommand};
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use yew_ssg_lib::error::{BuildError, ConfigError, GeneratorError, InitError, TemplateError};
use yew_ssg_lib::{BuildReport, InitOptions, InitReport, InitScaffolder, SiteBuilder};

/// A Yew-based static site generator.
///
/// yew-ssg turns Markdown content and Tera templates into a fully static website.
/// Configuration is read from site.toml in the site root directory.
///
/// Quick start:
///   yew-ssg init my-site          # scaffold a new site
///   cd my-site
///   yew-ssg build                  # generate output in dist/
#[derive(Parser)]
#[command(name = "yew-ssg")]
#[command(about = "A Yew-based static site generator")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Build the static site from Markdown content and templates.
    ///
    /// Reads site.toml from DIR, discovers all .md files in the content
    /// directory, renders them through Tera HTML templates, compiles SCSS
    /// stylesheets, copies static files, and writes the result to the output
    /// directory (default: dist/).
    ///
    /// Examples:
    ///   yew-ssg build
    ///   yew-ssg build --verbose
    ///   yew-ssg build --dir ./my-site --include-drafts
    ///   yew-ssg build --clean --verbose
    ///   yew-ssg build --dry-run
    ///   yew-ssg build --output /tmp/preview
    Build {
        /// Root directory of the site (must contain site.toml).
        ///
        /// Defaults to the current working directory. Use this when running
        /// the tool from outside the site directory.
        #[arg(short, long, default_value = ".", value_name = "PATH")]
        dir: PathBuf,

        /// Print detailed progress for each build stage.
        ///
        /// Shows route discovery, template loading, per-page rendering,
        /// asset processing, and file writing. Mutually exclusive with --quiet.
        #[arg(short, long, conflicts_with = "quiet")]
        verbose: bool,

        /// Suppress all output except errors.
        ///
        /// Only error messages are printed; the build summary is suppressed.
        /// Mutually exclusive with --verbose.
        #[arg(short, long, conflicts_with = "verbose")]
        quiet: bool,

        /// Include pages marked `draft = true` in frontmatter.
        ///
        /// Draft pages are excluded by default, allowing you to commit
        /// works-in-progress without publishing them.
        #[arg(long)]
        include_drafts: bool,

        /// Simulate the build without writing any output files.
        ///
        /// All build stages run normally (content is parsed, templates are
        /// rendered) but no files are created or modified. Useful for
        /// checking for errors before a real build.
        #[arg(long)]
        dry_run: bool,

        /// Remove all files from the output directory before building.
        ///
        /// Equivalent to running `yew-ssg clean` then `yew-ssg build`.
        #[arg(long)]
        clean: bool,

        /// Override the output directory from site.toml.
        ///
        /// Useful for generating a preview build in a temporary location
        /// without permanently changing site.toml.
        #[arg(short = 'o', long, value_name = "PATH")]
        output: Option<PathBuf>,
    },

    /// Remove all generated files from the output directory.
    ///
    /// Deletes the output directory configured in site.toml (default: dist/).
    /// Does not affect content, templates, or styles.
    ///
    /// Examples:
    ///   yew-ssg clean
    ///   yew-ssg clean --dir ./my-site
    Clean {
        /// Root directory of the site (must contain site.toml).
        #[arg(short, long, default_value = ".", value_name = "PATH")]
        dir: PathBuf,
    },

    /// Initialize a new site with a default directory structure.
    ///
    /// Creates the following layout in PATH (defaults to the current directory):
    ///
    ///   site.toml               site configuration
    ///   content/_index.md       home page content
    ///   templates/base.html     base HTML layout
    ///   templates/page.html     single-page template
    ///   templates/section.html  section/listing template
    ///   styles/main.scss        starter stylesheet
    ///   static/scripts.js       placeholder scripts file
    ///   static/favicon.png      placeholder favicon
    ///
    /// Examples:
    ///   yew-ssg init
    ///   yew-ssg init my-site
    ///   yew-ssg init my-site --name "My Blog" --base-url "https://myblog.com"
    ///   yew-ssg init my-site --force
    Init {
        /// Directory to initialize (defaults to the current directory).
        ///
        /// The directory will be created if it does not exist.
        #[arg(default_value = ".", value_name = "PATH")]
        path: PathBuf,

        /// Site name used in templates and site.toml.
        ///
        /// Defaults to the directory name, or "My Site" when initializing
        /// the current directory.
        #[arg(short, long, value_name = "NAME")]
        name: Option<String>,

        /// Base URL for the site (must start with http:// or https://).
        ///
        /// Used in site.toml and available as `{{ site.base_url }}` in templates.
        /// Defaults to https://example.com.
        #[arg(short = 'u', long, value_name = "URL")]
        base_url: Option<String>,

        /// Initialize even if the directory is not empty.
        ///
        /// Without this flag, yew-ssg will prompt for confirmation before
        /// initializing a non-empty directory. Existing files are never
        /// overwritten.
        #[arg(short, long)]
        force: bool,

        /// Initialize with islands support (Yew/WASM hydration).
        ///
        /// When set, the generated templates will include WASM hydration
        /// script tags for interactive Yew components.
        #[arg(long)]
        islands: bool,
    },

    /// List all routes that would be discovered from the content directory.
    ///
    /// Reads site.toml and walks the content directory to show which URL paths
    /// would be generated, which content files they map to, and which output
    /// files they would produce. No files are written.
    ///
    /// Examples:
    ///   yew-ssg routes
    ///   yew-ssg routes --dir ./my-site
    Routes {
        /// Root directory of the site (must contain site.toml).
        #[arg(short, long, default_value = ".", value_name = "PATH")]
        dir: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();

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

                    if report.has_warnings() {
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
                println!("✓ Output directory cleaned.");
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
            islands,
        } => {
            match run_init(&InitArgs {
                path,
                name,
                base_url,
                force,
                islands,
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
    }
}

// ---------------------------------------------------------------------------
// Build
// ---------------------------------------------------------------------------

struct BuildArgs {
    dir: PathBuf,
    verbose: bool,
    include_drafts: bool,
    dry_run: bool,
    clean: bool,
    output: Option<PathBuf>,
}

#[allow(clippy::result_large_err)]
fn run_build(args: &BuildArgs) -> Result<BuildReport, GeneratorError> {
    // Load config so we can apply the --output override before building
    let mut config = yew_ssg_lib::SiteConfig::from_dir(&args.dir)?;
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

#[allow(clippy::result_large_err)]
fn run_clean(dir: &Path) -> Result<(), GeneratorError> {
    SiteBuilder::from_dir(dir)?.clean()
}

// ---------------------------------------------------------------------------
// Init
// ---------------------------------------------------------------------------

struct InitArgs {
    path: PathBuf,
    name: Option<String>,
    base_url: Option<String>,
    force: bool,
    islands: bool,
}

#[allow(clippy::result_large_err)]
fn run_init(args: &InitArgs) -> Result<InitReport, GeneratorError> {
    use yew_ssg_lib::init::{derive_site_name, is_directory_empty};

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

#[allow(clippy::result_large_err)]
fn run_routes(dir: &Path) -> Result<(), GeneratorError> {
    use yew_ssg_lib::{RouteDiscovery, SiteConfig};

    let config = SiteConfig::from_dir(dir)?;
    let discovery = RouteDiscovery::new(&config.build.content_dir);
    let registry = discovery.discover()?;

    println!(
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

// ---------------------------------------------------------------------------
// Error rendering
// ---------------------------------------------------------------------------

/// Print a user-friendly error message with a contextual hint.
fn render_error(e: &GeneratorError) {
    eprintln!("\n✗ Error: {e}");

    let hint: Option<&str> = match e {
        GeneratorError::Config(ConfigError::NotFound(_)) => Some(
            "Run 'yew-ssg init' to create a new site, or use --dir to point to your site directory.",
        ),
        GeneratorError::Build(BuildError::NoContent) => {
            Some("Add .md files to your content/ directory. Start with content/_index.md.")
        }
        GeneratorError::Template(TemplateError::NotFound(_)) => Some(
            "Check that your templates/ directory exists and contains base.html and page.html.",
        ),
        GeneratorError::Template(TemplateError::DirNotFound(_)) => Some(
            "Check that your templates/ directory exists. Run 'yew-ssg init' to create a default site.",
        ),
        GeneratorError::Init(InitError::Cancelled) => {
            // Silent — user intentionally cancelled
            return;
        }
        _ => None,
    };

    if let Some(hint) = hint {
        eprintln!("  Hint: {hint}");
    }

    eprintln!();
}
