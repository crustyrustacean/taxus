// generator/src/bin/taxus/cli.rs

use clap::{Parser, Subcommand};
use std::net::IpAddr;
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
    ///   taxus init my-site --name "My Blog" --base-url `<https://myblog.com>`
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
        /// Defaults to <https://example.com>.
        #[arg(short = 'u', long, value_name = "URL")]
        base_url: Option<String>,

        /// Initialize even if the directory is not empty.
        ///
        /// Without this flag, taxus will prompt for confirmation before
        /// initializing a non-empty directory. Existing files are never
        /// overwritten.
        #[arg(short, long)]
        force: bool,

        /// Disable islands support for a plain Tera/Markdown scaffold.
        ///
        /// By default, taxus initializes a site with WASM islands enabled: the
        /// generated `base.html` includes the WASM hydration script and the
        /// section template demonstrates a `Counter` island. Pass this flag to
        /// generate a plain scaffold with no Yew/WASM hydration.
        #[arg(long)]
        no_islands: bool,
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
    /// By default the server listens on 127.0.0.1 only, so it is reachable
    /// from this machine and nothing else. Pass --host 0.0.0.0 to expose it
    /// on your local network, e.g. to test the site on a phone.
    ///
    /// Examples:
    ///   taxus serve
    ///   taxus serve --dir ./my-site
    ///   taxus serve --port 8080
    ///   taxus serve --host 0.0.0.0
    ///   taxus serve --dir ./my-site --open
    Serve {
        /// Root directory of the site (must contain site.toml).
        #[arg(short, long, default_value = ".", value_name = "PATH")]
        dir: PathBuf,

        /// IP address to listen on.
        ///
        /// Defaults to loopback (127.0.0.1) so the server is only reachable
        /// from this machine. Use 0.0.0.0 (or :: for IPv6) to accept
        /// connections from other devices on your network.
        #[arg(long, default_value = "127.0.0.1", value_name = "ADDR")]
        host: IpAddr,

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    fn parse_serve(args: &[&str]) -> Result<(IpAddr, u16), clap::Error> {
        let cli = Cli::try_parse_from(args)?;
        match cli.command {
            Commands::Serve { host, port, .. } => Ok((host, port)),
            _ => panic!("expected the serve subcommand"),
        }
    }

    #[test]
    fn serve_defaults_to_loopback_host() {
        let (host, port) = parse_serve(&["taxus", "serve"]).unwrap();
        assert_eq!(host, IpAddr::V4(Ipv4Addr::LOCALHOST));
        assert_eq!(port, 3000);
    }

    #[test]
    fn serve_accepts_explicit_host() {
        let (host, _) = parse_serve(&["taxus", "serve", "--host", "0.0.0.0"]).unwrap();
        assert_eq!(host, IpAddr::V4(Ipv4Addr::UNSPECIFIED));

        let (host, port) =
            parse_serve(&["taxus", "serve", "--host", "::1", "--port", "8080"]).unwrap();
        assert_eq!(host, "::1".parse::<IpAddr>().unwrap());
        assert_eq!(port, 8080);
    }

    #[test]
    fn serve_rejects_invalid_host() {
        let err = parse_serve(&["taxus", "serve", "--host", "not-an-ip"]).unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::ValueValidation);
    }
}
