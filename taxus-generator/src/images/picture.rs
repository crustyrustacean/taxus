use super::processor::ProcessedImage;

pub fn render_picture(processed: &ProcessedImage, alt: &str, loading: &str) -> String {
    let srcset = processed.srcset();
    let fallback_src = processed.fallback_src();
    let mime_type = processed.mime_type();
    let width = processed.meta.original_width;
    let height = processed.meta.original_height;

    format!(
        r#"<picture>
  <source srcset="{}"
          type="{}">
  <img src="{}"
       alt="{}"
       width="{}" height="{}"
       loading="{}"
       decoding="async">
</picture>"#,
        srcset, mime_type, fallback_src, alt, width, height, loading
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ImageConfig;
    use std::path::{Path, PathBuf};
    use tempfile::TempDir;

    fn create_test_image(dir: &Path, name: &str, width: u32, height: u32) -> PathBuf {
        let path = dir.join(name);
        let img = image::RgbImage::from_pixel(width, height, image::Rgb([128, 128, 128]));
        img.save(&path).unwrap();
        path
    }

    fn create_processed_image(width: u32, height: u32) -> (TempDir, ProcessedImage) {
        let temp = TempDir::new().unwrap();
        let source = create_test_image(temp.path(), "hero.jpg", width, height);
        let output_dir = temp.path().join("dist");
        let processor = crate::images::ImageProcessor::new(ImageConfig::default(), output_dir);
        let processed = processor.process(&source, "Test alt").unwrap();
        (temp, processed)
    }

    #[test]
    fn test_picture_contains_source_element() {
        let (_temp, processed) = create_processed_image(1600, 900);
        let html = render_picture(&processed, "Hero image", "eager");

        assert!(html.contains("<picture>"));
        assert!(html.contains("</picture>"));
        assert!(html.contains("<source"));
        assert!(html.contains("type=\"image/webp\""));
    }

    #[test]
    fn test_picture_contains_img_fallback() {
        let (_temp, processed) = create_processed_image(1600, 900);
        let html = render_picture(&processed, "Hero image", "eager");

        assert!(html.contains("<img"));
        assert!(html.contains("alt=\"Hero image\""));
    }

    #[test]
    fn test_picture_width_height_from_original() {
        let (_temp, processed) = create_processed_image(1600, 900);
        let html = render_picture(&processed, "Test", "eager");

        assert!(html.contains("width=\"1600\""));
        assert!(html.contains("height=\"900\""));
    }

    #[test]
    fn test_picture_loading_eager() {
        let (_temp, processed) = create_processed_image(1600, 900);
        let html = render_picture(&processed, "Test", "eager");

        assert!(html.contains("loading=\"eager\""));
    }

    #[test]
    fn test_picture_loading_lazy() {
        let (_temp, processed) = create_processed_image(1600, 900);
        let html = render_picture(&processed, "Test", "lazy");

        assert!(html.contains("loading=\"lazy\""));
    }

    #[test]
    fn test_picture_srcset_contains_all_widths() {
        let (_temp, processed) = create_processed_image(1600, 900);
        let html = render_picture(&processed, "Test", "eager");

        assert!(html.contains("400w"));
        assert!(html.contains("800w"));
        assert!(html.contains("1200w"));
    }

    #[test]
    fn test_picture_fallback_uses_middle_variant() {
        let (_temp, processed) = create_processed_image(1600, 900);
        let html = render_picture(&processed, "Test", "eager");

        let src_match = html.find("src=\"").unwrap();
        let src_section = &html[src_match..src_match + 100];
        assert!(
            src_section.contains("800w"),
            "Fallback src should use middle (800w) variant"
        );
    }

    #[test]
    fn test_picture_decoding_async() {
        let (_temp, processed) = create_processed_image(1600, 900);
        let html = render_picture(&processed, "Test", "eager");

        assert!(html.contains("decoding=\"async\""));
    }
}
