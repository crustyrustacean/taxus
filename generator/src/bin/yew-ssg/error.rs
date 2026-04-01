// src/generator/src/bin/error.rs

use yew_ssg_lib::error::{BuildError, ConfigError, GeneratorError, InitError, TemplateError};

// ---------------------------------------------------------------------------
// Error rendering
// ---------------------------------------------------------------------------

/// Print a user-friendly error message with a contextual hint.
pub fn render_error(e: &GeneratorError) {
    // Log structured error for debugging/monitoring
    tracing::error!(error = %e, error_type = std::any::type_name_of_val(e), "Build failed");

    // Print user-friendly error message
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
        tracing::warn!(hint = hint, "Suggestion");
        tracing::error!("  Hint: {hint}");
    }
}
