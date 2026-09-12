//! Hero image processing (emit phase).
//!
//! A hero image named in frontmatter is resized into the configured
//! widths, converted to the configured format, and described to templates
//! as `page.hero`. Variant files are named by a hash of the image bytes
//! and the effective quality, so unchanged images keep stable URLs and a
//! quality change re-encodes; see the book's
//! [Decisions](https://crustyrustacean.github.io/taxus/theory/decisions.html) chapter.

pub mod picture;
pub mod processor;
pub mod registry;

pub use picture::render_picture;
pub use processor::{
    ImageMeta, ImageProcessor, ImageVariant, LOSSY_WEBP_AVAILABLE, ProcessedImage,
};
pub use registry::ImageRegistry;
