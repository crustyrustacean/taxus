// generator/src/bin/taxus/cli.rs

use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// A static site generator founded in Tera with WebAssembly islands.
///
/// taxus turns Markdown content and Tera templates into a fully static website.
/// Configuration is read from site.toml in the site root directory.
///
/// Quick start:
///   taxus init my-site          # scaffold a new site
///   cd my-site
///   taxus build                  # generate output in dist/
#[derive(Parser)]
#[command(name = "taxus")]
#[command(about = "A Yew-based static site generator")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Build the static site from Markdown content and templates.
    ///
    /// Reads site.toml from DIR, discovers all .md files in the content
    /// directory, renders them through Tera HTML templates, compiles SCSS
    /// stylesheets, copies static files, and writes the result to the output
    /// directory (default: dist/).
    ///
    /// Examples:
    ///   taxus build
    ///   taxus build --verbose
    ///   taxus build --dir ./my-site --include-drafts
    ///   taxus build --clean --verbose
    ///   taxus build --dry-run
    ///   taxus build --output /tmp/preview
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
        /// Equivalent to running `taxus clean` then `taxus build`.
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
    ///   taxus clean
    ///   taxus clean --dir ./my-site
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
    ///   taxus init
    ///   taxus init my-site
    ///   taxus init my-site --name "My Blog" --base-url "https://myblog.com"
    ///   taxus init my-site --force
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
        /// Without this flag, taxus will prompt for confirmation before
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
    ///   taxus routes
    ///   taxus routes --dir ./my-site
    Routes {
        /// Root directory of the site (must contain site.toml).
        #[arg(short, long, default_value = ".", value_name = "PATH")]
        dir: PathBuf,
    },

    /// Start a development server with live reload.
    ///
    /// Serves the output directory and watches for file changes. When content,
    /// templates, styles, or configuration files change, the site is rebuilt
    /// and connected browsers are automatically reloaded via WebSocket.
    ///
    /// Examples:
    ///   taxus serve
    ///   taxus serve .
    ///   taxus serve --port 8080
    ///   taxus serve ./my-site
    Serve {
        /// Root directory of the site (must contain site.toml).
        #[arg(default_value = ".", value_name = "PATH")]
        dir: PathBuf,

        /// Port to listen on.
        #[arg(short, long, default_value = "3000")]
        port: u16,

        /// Print detailed progress for each build stage.
        #[arg(short, long, conflicts_with = "quiet")]
        verbose: bool,

        /// Suppress all output except errors.
        #[arg(short, long, conflicts_with = "verbose")]
        quiet: bool,

        /// Open the site in a browser after starting the server.
        #[arg(short, long)]
        open: bool,
    },
}
