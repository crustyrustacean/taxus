//! Template rendering module.
//!
//! This module provides a flexible template system with a trait-based
//! backend, allowing different template engines to be used. The primary
//! implementation uses [Tera](https://tera.netlify.app/), a Jinja2-like
//! template engine for Rust.
//!
//! # Overview
//!
//! The template system consists of three main components:
//!
//! - [`TemplateRenderer`] - A trait defining the template rendering interface
//! - [`TeraRenderer`] - The primary implementation using Tera
//! - [`TemplateContext`] - Context types containing variables for templates
//!
//! # Example
//!
//! ```no_run
//! use yew_ssg_lib::templates::{TemplateRenderer, TeraRenderer, TemplateContext, SiteContext, PageContext};
//!
//! // Create a renderer and load templates
//! let mut renderer = TeraRenderer::from_dir("templates")?;
//!
//! // Create context with site and page data
//! let site = SiteContext {
//!     name: "My Site".to_string(),
//!     base_url: "https://example.com".to_string(),
//!     description: None,
//!     author: None,
//! };
//! let page = PageContext {
//!     title: "Hello".to_string(),
//!     description: None,
//!     path: "/hello/".to_string(),
//!     content: "<p>World</p>".to_string(),
//!     raw_content: "World".to_string(),
//!     date: None,
//!     draft: false,
//! };
//! let ctx = TemplateContext::new(site).with_page(page);
//!
//! // Render a template
//! let html = renderer.render("page.html", &ctx)?;
//! # Ok::<(), yew_ssg_lib::error::TemplateError>(())
//! ```

mod context;
mod renderer;

pub use context::{PageContext, SectionContext, SiteContext, TemplateContext};
pub use renderer::{TemplateRenderer, TeraRenderer};
