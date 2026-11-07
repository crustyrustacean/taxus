//! Compile-only consumer fixture for the public `taxus` crate name.

use taxus::{SiteBuilder, SiteConfig};

fn accepts_public_api(config: SiteConfig) {
    let builder = SiteBuilder::new(config);
    let _: &SiteConfig = builder.config();
}

fn main() {
    let _ = accepts_public_api;
}
