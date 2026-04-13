use crate::config::ImageConfig;
use crate::error::{ImageError, Result};
use image::GenericImageView;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ImageVariant {
    pub path: PathBuf,
    pub width: u32,
    pub height: u32,
    pub file_size: u64,
}

#[derive(Debug, Clone)]
pub struct ImageMeta {
    pub original_width: u32,
    pub original_height: u32,
    pub aspect_ratio: f64,
    pub alt: String,
    pub variants: Vec<ImageVariant>,
}

#[derive(Debug, Clone)]
pub struct ProcessedImage {
    pub source_path: PathBuf,
    pub output_dir: PathBuf,
    pub meta: ImageMeta,
    pub format: String,
}

impl ProcessedImage {
    pub fn mime_type(&self) -> String {
        match self.format.as_str() {
            "webp" => "image/webp".to_string(),
            "jpeg" | "jpg" => "image/jpeg".to_string(),
            "png" => "image/png".to_string(),
            _ => "image/webp".to_string(),
        }
    }

    pub fn extension(&self) -> &str {
        match self.format.as_str() {
            "jpeg" | "jpg" => "jpg",
            "png" => "png",
            _ => "webp",
        }
    }

    pub fn url_path(&self, variant: &ImageVariant) -> String {
        let rel = variant
            .path
            .strip_prefix(&self.output_dir)
            .unwrap_or(&variant.path);
        format!("/{}", rel.to_string_lossy().replace('\\', "/"))
    }

    pub fn srcset(&self) -> String {
        self.meta
            .variants
            .iter()
            .map(|v| {
                let url = self.url_path(v);
                format!("{} {}w", url, v.width)
            })
            .collect::<Vec<_>>()
            .join(",\n                  ")
    }

    pub fn fallback_src(&self) -> String {
        let mid = self.meta.variants.len() / 2;
        let variant = self
            .meta
            .variants
            .get(mid)
            .unwrap_or_else(|| &self.meta.variants[0]);
        self.url_path(variant)
    }
}

pub struct ImageProcessor {
    config: ImageConfig,
    output_dir: PathBuf,
}

impl ImageProcessor {
    pub fn new(config: ImageConfig, output_dir: PathBuf) -> Self {
        Self { config, output_dir }
    }

    fn extension(&self) -> &str {
        match self.config.format.as_str() {
            "jpeg" | "jpg" => "jpg",
            "png" => "png",
            _ => "webp",
        }
    }

    pub fn process(&self, source: &Path, alt: &str) -> Result<ProcessedImage> {
        if !source.exists() {
            return Err(ImageError::NotFound(source.to_path_buf()).into());
        }

        let image_output_dir = self.output_dir.join(&self.config.output_dir);
        let hash = Self::compute_hash(source);

        let prefix = Self::compute_prefix(source);

        let expected_variants: Vec<(u32, u32, PathBuf)> = self
            .config
            .widths
            .iter()
            .map(|&target_width| {
                let (w, h) = self.variant_dimensions(source, target_width);
                let filename = format!("{}-{}-{}w.{}", prefix, hash, w, self.extension());
                (w, h, image_output_dir.join(&filename))
            })
            .collect();

        if Self::all_variants_exist(&expected_variants) {
            return Ok(Self::build_from_cache(
                source,
                &image_output_dir,
                &self.output_dir,
                alt,
                &hash,
                expected_variants,
            ));
        }

        let img = image::open(source).map_err(|e| ImageError::DecodeFailed {
            path: source.to_path_buf(),
            reason: e.to_string(),
        })?;

        let (original_width, original_height) = img.dimensions();
        let aspect_ratio = original_width as f64 / original_height as f64;

        std::fs::create_dir_all(&image_output_dir).map_err(|e| ImageError::Io {
            path: image_output_dir.clone(),
            source: e,
        })?;

        let mut variants = Vec::new();

        for &target_width in &self.config.widths {
            if target_width >= original_width {
                let variant_width = original_width;
                let variant_height = original_height;
                let filename = format!(
                    "{}-{}-{}w.{}",
                    prefix,
                    hash,
                    variant_width,
                    self.extension()
                );
                let out_path = image_output_dir.join(&filename);

                let mut buf = std::io::Cursor::new(Vec::new());
                self.encode(&img, &mut buf)?;
                let data = buf.into_inner();

                std::fs::write(&out_path, &data).map_err(|e| ImageError::Io {
                    path: out_path.clone(),
                    source: e,
                })?;

                variants.push(ImageVariant {
                    path: out_path,
                    width: variant_width,
                    height: variant_height,
                    file_size: data.len() as u64,
                });
            } else {
                let variant_height = (target_width as f64 / aspect_ratio).round() as u32;
                let resized = img.resize(
                    target_width,
                    variant_height,
                    image::imageops::FilterType::Lanczos3,
                );
                let filename =
                    format!("{}-{}-{}w.{}", prefix, hash, target_width, self.extension());
                let out_path = image_output_dir.join(&filename);

                let mut buf = std::io::Cursor::new(Vec::new());
                self.encode(&resized, &mut buf)?;
                let data = buf.into_inner();

                std::fs::write(&out_path, &data).map_err(|e| ImageError::Io {
                    path: out_path.clone(),
                    source: e,
                })?;

                variants.push(ImageVariant {
                    path: out_path,
                    width: target_width,
                    height: variant_height,
                    file_size: data.len() as u64,
                });
            }
        }

        let source_relative = source.to_path_buf();

        Ok(ProcessedImage {
            source_path: source_relative,
            output_dir: self.output_dir.clone(),
            meta: ImageMeta {
                original_width,
                original_height,
                aspect_ratio,
                alt: alt.to_string(),
                variants,
            },
            format: self.config.format.clone(),
        })
    }

    pub fn process_dry(&self, source: &Path, alt: &str) -> Result<ProcessedImage> {
        if !source.exists() {
            return Err(ImageError::NotFound(source.to_path_buf()).into());
        }

        let img = image::open(source).map_err(|e| ImageError::DecodeFailed {
            path: source.to_path_buf(),
            reason: e.to_string(),
        })?;

        let (original_width, original_height) = img.dimensions();
        let aspect_ratio = original_width as f64 / original_height as f64;

        let image_output_dir = self.output_dir.join(&self.config.output_dir);
        let hash = Self::compute_hash(source);

        let prefix = Self::compute_prefix(source);

        let mut variants = Vec::new();

        for &target_width in &self.config.widths {
            let (variant_width, variant_height) = if target_width >= original_width {
                (original_width, original_height)
            } else {
                (
                    target_width,
                    (target_width as f64 / aspect_ratio).round() as u32,
                )
            };

            let filename = format!(
                "{}-{}-{}w.{}",
                prefix,
                hash,
                variant_width,
                self.extension()
            );
            let out_path = image_output_dir.join(&filename);

            variants.push(ImageVariant {
                path: out_path,
                width: variant_width,
                height: variant_height,
                file_size: 0,
            });
        }

        Ok(ProcessedImage {
            source_path: source.to_path_buf(),
            output_dir: self.output_dir.clone(),
            meta: ImageMeta {
                original_width,
                original_height,
                aspect_ratio,
                alt: alt.to_string(),
                variants,
            },
            format: self.config.format.clone(),
        })
    }

    fn encode(&self, img: &image::DynamicImage, buf: &mut std::io::Cursor<Vec<u8>>) -> Result<()> {
        match self.config.format.as_str() {
            "webp" => {
                img.write_to(buf, image::ImageFormat::WebP)
                    .map_err(|e| ImageError::EncodeFailed(e.to_string()))?;
            }
            "jpeg" | "jpg" => {
                img.write_to(buf, image::ImageFormat::Jpeg)
                    .map_err(|e| ImageError::EncodeFailed(e.to_string()))?;
            }
            "png" => {
                img.write_to(buf, image::ImageFormat::Png)
                    .map_err(|e| ImageError::EncodeFailed(e.to_string()))?;
            }
            _ => {
                img.write_to(buf, image::ImageFormat::WebP)
                    .map_err(|e| ImageError::EncodeFailed(e.to_string()))?;
            }
        }
        Ok(())
    }

    fn variant_dimensions(&self, source: &Path, target_width: u32) -> (u32, u32) {
        if let Ok(img) = image::image_dimensions(source) {
            let (original_width, original_height) = img;
            let aspect_ratio = original_width as f64 / original_height as f64;
            if target_width >= original_width {
                (original_width, original_height)
            } else {
                (
                    target_width,
                    (target_width as f64 / aspect_ratio).round() as u32,
                )
            }
        } else {
            (target_width, target_width)
        }
    }

    fn all_variants_exist(expected: &[(u32, u32, PathBuf)]) -> bool {
        expected.iter().all(|(_, _, path)| path.exists())
    }

    fn build_from_cache(
        source: &Path,
        _image_output_dir: &Path,
        base_output_dir: &Path,
        alt: &str,
        _hash: &str,
        expected: Vec<(u32, u32, PathBuf)>,
    ) -> ProcessedImage {
        let original_dims =
            image::image_dimensions(source).unwrap_or((expected[0].0, expected[0].1));
        let (original_width, original_height) = original_dims;
        let aspect_ratio = original_width as f64 / original_height as f64;

        let format = Self::format_from_filename(&expected);

        let variants: Vec<ImageVariant> = expected
            .into_iter()
            .map(|(w, h, path)| {
                let file_size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                ImageVariant {
                    path,
                    width: w,
                    height: h,
                    file_size,
                }
            })
            .collect();

        ProcessedImage {
            source_path: source.to_path_buf(),
            output_dir: base_output_dir.to_path_buf(),
            meta: ImageMeta {
                original_width,
                original_height,
                aspect_ratio,
                alt: alt.to_string(),
                variants,
            },
            format,
        }
    }

    fn format_from_filename(expected: &[(u32, u32, PathBuf)]) -> String {
        expected
            .first()
            .and_then(|(_, _, path)| path.extension())
            .and_then(|ext| ext.to_str())
            .map(|ext| match ext {
                "jpg" | "jpeg" => "jpeg".to_string(),
                "png" => "png".to_string(),
                _ => "webp".to_string(),
            })
            .unwrap_or_else(|| "webp".to_string())
    }

    fn compute_prefix(source: &Path) -> String {
        let stem = source
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("image");
        stem.to_string()
    }

    fn compute_hash(source: &Path) -> String {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        source.hash(&mut hasher);
        if let Ok(metadata) = std::fs::metadata(source) {
            if let Ok(modified) = metadata.modified()
                && let Ok(duration) = modified.duration_since(std::time::UNIX_EPOCH)
            {
                duration.as_nanos().hash(&mut hasher);
            }
            metadata.len().hash(&mut hasher);
        }
        let hash = hasher.finish();
        format!("{:x}", hash)[..6].to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_image(dir: &Path, name: &str, width: u32, height: u32) -> PathBuf {
        let path = dir.join(name);
        let img = image::RgbImage::from_pixel(width, height, image::Rgb([128, 128, 128]));
        img.save(&path).unwrap();
        path
    }

    fn default_config() -> ImageConfig {
        ImageConfig::default()
    }

    #[test]
    fn test_processor_creates_variants() {
        let temp = TempDir::new().unwrap();
        let source = create_test_image(temp.path(), "hero.jpg", 1600, 900);
        let output_dir = temp.path().join("dist");

        let processor = ImageProcessor::new(default_config(), output_dir);
        let result = processor.process(&source, "Test alt").unwrap();

        assert_eq!(result.meta.variants.len(), 3);
        assert_eq!(result.meta.original_width, 1600);
        assert_eq!(result.meta.original_height, 900);
    }

    #[test]
    fn test_processor_resize_preserves_aspect_ratio() {
        let temp = TempDir::new().unwrap();
        let source = create_test_image(temp.path(), "hero.jpg", 1600, 900);
        let output_dir = temp.path().join("dist");

        let processor = ImageProcessor::new(default_config(), output_dir);
        let result = processor.process(&source, "Test alt").unwrap();

        let variant_400 = &result.meta.variants[0];
        assert_eq!(variant_400.width, 400);
        let expected_height = (400.0_f64 / (1600.0_f64 / 900.0_f64)).round() as u32;
        assert_eq!(variant_400.height, expected_height);
    }

    #[test]
    fn test_processor_skips_when_smaller() {
        let temp = TempDir::new().unwrap();
        let source = create_test_image(temp.path(), "hero.jpg", 300, 200);
        let output_dir = temp.path().join("dist");

        let processor = ImageProcessor::new(default_config(), output_dir);
        let result = processor.process(&source, "Test alt").unwrap();

        assert_eq!(result.meta.variants.len(), 3);
        for variant in &result.meta.variants {
            assert_eq!(variant.width, 300);
            assert_eq!(variant.height, 200);
        }
    }

    #[test]
    fn test_processor_hash_based_filenames() {
        let temp = TempDir::new().unwrap();
        let source = create_test_image(temp.path(), "hero.jpg", 1600, 900);
        let output_dir = temp.path().join("dist");

        let processor = ImageProcessor::new(default_config(), output_dir);
        let result = processor.process(&source, "Test alt").unwrap();

        for variant in &result.meta.variants {
            let filename = variant.path.file_name().unwrap().to_str().unwrap();
            assert!(filename.starts_with("hero-"));
            assert!(
                filename.contains("-400w.")
                    || filename.contains("-800w.")
                    || filename.contains("-1200w.")
            );
            assert!(filename.ends_with(".webp"));
        }
    }

    #[test]
    fn test_processor_not_found() {
        let temp = TempDir::new().unwrap();
        let output_dir = temp.path().join("dist");
        let source = temp.path().join("nonexistent.jpg");

        let processor = ImageProcessor::new(default_config(), output_dir);
        let result = processor.process(&source, "Test alt");
        assert!(result.is_err());
    }

    #[test]
    fn test_processed_image_mime_type() {
        let temp = TempDir::new().unwrap();
        let source = create_test_image(temp.path(), "hero.jpg", 1600, 900);
        let output_dir = temp.path().join("dist");

        let processor = ImageProcessor::new(default_config(), output_dir);
        let result = processor.process(&source, "Test alt").unwrap();

        assert_eq!(result.mime_type(), "image/webp");
    }

    #[test]
    fn test_processed_image_jpeg_format() {
        let temp = TempDir::new().unwrap();
        let source = create_test_image(temp.path(), "hero.jpg", 1600, 900);
        let output_dir = temp.path().join("dist");

        let config = ImageConfig {
            format: "jpeg".to_string(),
            ..Default::default()
        };
        let processor = ImageProcessor::new(config, output_dir);
        let result = processor.process(&source, "Test alt").unwrap();

        assert_eq!(result.mime_type(), "image/jpeg");
        assert_eq!(result.extension(), "jpg");
    }

    #[test]
    fn test_processed_image_srcset() {
        let temp = TempDir::new().unwrap();
        let source = create_test_image(temp.path(), "hero.jpg", 1600, 900);
        let output_dir = temp.path().join("dist");

        let processor = ImageProcessor::new(default_config(), output_dir);
        let result = processor.process(&source, "Test alt").unwrap();

        let srcset = result.srcset();
        assert!(srcset.contains("400w"));
        assert!(srcset.contains("800w"));
        assert!(srcset.contains("1200w"));
        assert!(srcset.contains("/images/"));
    }

    #[test]
    fn test_processed_image_fallback_src_uses_middle_variant() {
        let temp = TempDir::new().unwrap();
        let source = create_test_image(temp.path(), "hero.jpg", 1600, 900);
        let output_dir = temp.path().join("dist");

        let processor = ImageProcessor::new(default_config(), output_dir);
        let result = processor.process(&source, "Test alt").unwrap();

        let fallback = result.fallback_src();
        assert!(
            fallback.contains("800w"),
            "Fallback should use middle variant, got: {}",
            fallback
        );
    }

    #[test]
    fn test_processor_dry_run_no_files() {
        let temp = TempDir::new().unwrap();
        let source = create_test_image(temp.path(), "hero.jpg", 1600, 900);
        let output_dir = temp.path().join("dist");

        let processor = ImageProcessor::new(default_config(), output_dir);
        let result = processor.process_dry(&source, "Test alt").unwrap();

        assert_eq!(result.meta.variants.len(), 3);
        for variant in &result.meta.variants {
            assert!(!variant.path.exists());
        }
    }

    #[test]
    fn test_processor_partial_skip() {
        let temp = TempDir::new().unwrap();
        let source = create_test_image(temp.path(), "hero.jpg", 600, 400);
        let output_dir = temp.path().join("dist");

        let processor = ImageProcessor::new(default_config(), output_dir);
        let result = processor.process(&source, "Test alt").unwrap();

        assert_eq!(result.meta.variants.len(), 3);
        assert_eq!(result.meta.variants[0].width, 400);
        assert_eq!(
            result.meta.variants[0].height,
            (400.0_f64 / (600.0_f64 / 400.0_f64)).round() as u32
        );
        assert_eq!(result.meta.variants[1].width, 600);
        assert_eq!(result.meta.variants[1].height, 400);
        assert_eq!(result.meta.variants[2].width, 600);
        assert_eq!(result.meta.variants[2].height, 400);
    }
}
