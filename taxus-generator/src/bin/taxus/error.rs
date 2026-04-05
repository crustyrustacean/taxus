// src/generator/bin/taxus/error.rs

use taxus_lib::error::{ConfigError, GeneratorError, InitError, TemplateError};

// ---------------------------------------------------------------------------
// Error rendering
// ---------------------------------------------------------------------------

/// Print a user-friendly error message with a contextual hint.
pub fn render_error(e: &GeneratorError) {
    tracing::error!(error = %e, error_type = std::any::type_name_of_val(e), "Build failed");

    eprintln!("\n✗ Error: {e}");

    let hint: Option<&str> = match e {
        GeneratorError::Config(inner) if matches!(**inner, ConfigError::NotFound(_)) => Some(
            "Run 'taxus init' to create a new site, or use --dir to point to your site directory.",
        ),
        GeneratorError::NoContent => {
            Some("Add .md files to your content/ directory. Start with content/_index.md.")
        }
        GeneratorError::Template(inner) if matches!(**inner, TemplateError::NotFound(_)) => Some(
            "Check that your templates/ directory exists and contains base.html and page.html.",
        ),
        GeneratorError::Template(inner) if matches!(**inner, TemplateError::DirNotFound(_)) => {
            Some(
                "Check that your templates/ directory exists. Run 'taxus init' to create a default site.",
            )
        }
        GeneratorError::Init(inner) if matches!(**inner, InitError::Cancelled) => {
            return;
        }
        _ => None,
    };

    if let Some(hint) = hint {
        tracing::warn!(hint = hint, "Suggestion");
        tracing::error!("  Hint: {hint}");
    }
}
