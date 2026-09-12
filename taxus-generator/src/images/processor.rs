use crate::config::ImageConfig;
use crate::error::{ImageError, Result};
use image::GenericImageView;
use std::path::{Path, PathBuf};

/// Whether lossy WebP encoding (libwebp via the `webp` crate) is compiled in.
///
/// When `false`, WebP variants are written with the `image` crate's
/// lossless encoder and `images.quality` has no effect on them.
pub const LOSSY_WEBP_AVAILABLE: bool = cfg!(feature = "webp-lossy");

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
        let hash = self.compute_hash(source);

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
        let hash = self.compute_hash(source);

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

    /// The quality the encoder will actually apply.
    ///
    /// `None` for lossless output: PNG always, and WebP when the
    /// `webp-lossy` feature is disabled. Only an effective quality is folded
    /// into the cache key, so changing `images.quality` never invalidates
    /// variants it could not have changed.
    fn effective_quality(&self) -> Option<u8> {
        match self.config.format.as_str() {
            "jpeg" | "jpg" => Some(self.config.quality),
            "png" => None,
            _ if LOSSY_WEBP_AVAILABLE => Some(self.config.quality),
            _ => None,
        }
    }

    /// Whether `images.quality` will be silently ignored for this
    /// configuration because the output format is WebP and lossy WebP
    /// support was compiled out.
    pub fn quality_ignored_for_webp(&self) -> bool {
        !LOSSY_WEBP_AVAILABLE && !matches!(self.config.format.as_str(), "jpeg" | "jpg" | "png")
    }

    fn encode(&self, img: &image::DynamicImage, buf: &mut std::io::Cursor<Vec<u8>>) -> Result<()> {
        match self.config.format.as_str() {
            "jpeg" | "jpg" => {
                // JPEG has no alpha channel; drop it explicitly rather than
                // relying on the encoder's handling of RGBA input.
                let rgb;
                let img = if img.color().has_alpha() {
                    rgb = image::DynamicImage::ImageRgb8(img.to_rgb8());
                    &rgb
                } else {
                    img
                };
                let encoder =
                    image::codecs::jpeg::JpegEncoder::new_with_quality(buf, self.config.quality);
                img.write_with_encoder(encoder)
                    .map_err(|e| ImageError::EncodeFailed(e.to_string()))?;
            }
            // PNG is lossless; quality is ignored by design.
            "png" => {
                img.write_to(buf, image::ImageFormat::Png)
                    .map_err(|e| ImageError::EncodeFailed(e.to_string()))?;
            }
            _ => self.encode_webp(img, buf)?,
        }
        Ok(())
    }

    /// Lossy WebP via libwebp at `config.quality`. Alpha is preserved for
    /// RGBA sources; everything else is encoded as RGB.
    #[cfg(feature = "webp-lossy")]
    fn encode_webp(
        &self,
        img: &image::DynamicImage,
        buf: &mut std::io::Cursor<Vec<u8>>,
    ) -> Result<()> {
        use std::io::Write;

        let converted = if img.color().has_alpha() {
            image::DynamicImage::ImageRgba8(img.to_rgba8())
        } else {
            image::DynamicImage::ImageRgb8(img.to_rgb8())
        };
        let encoder = webp::Encoder::from_image(&converted)
            .map_err(|e| ImageError::EncodeFailed(format!("webp: {e}")))?;
        let encoded = encoder
            .encode_simple(false, f32::from(self.config.quality))
            .map_err(|e| ImageError::EncodeFailed(format!("webp: {e:?}")))?;
        buf.write_all(&encoded)
            .map_err(|e| ImageError::EncodeFailed(e.to_string()))?;
        Ok(())
    }

    /// Lossless WebP via the `image` crate; `config.quality` is ignored.
    #[cfg(not(feature = "webp-lossy"))]
    fn encode_webp(
        &self,
        img: &image::DynamicImage,
        buf: &mut std::io::Cursor<Vec<u8>>,
    ) -> Result<()> {
        img.write_to(buf, image::ImageFormat::WebP)
            .map_err(|e| ImageError::EncodeFailed(e.to_string()))?;
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

    /// Cache key for a source image: a digest of the file's bytes and the
    /// effective encoding quality (see [`Self::effective_quality`]).
    ///
    /// The same image therefore gets the same variant names on every
    /// machine and checkout, and changing `images.quality` re-encodes lossy
    /// variants. Path and mtime are deliberately not part of the key: they
    /// differ per clone, which made variant names — and every page linking
    /// them — differ between builds of identical content.
    ///
    /// A source that cannot be read hashes its path instead, so the key is
    /// always defined; the caller reports the read error when it decodes
    /// the image.
    fn compute_hash(&self, source: &Path) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        match std::fs::read(source) {
            Ok(bytes) => hasher.update(&bytes),
            Err(_) => hasher.update(source.to_string_lossy().as_bytes()),
        }
        // Two bytes so that "no effective quality" and "quality 0" differ.
        match self.effective_quality() {
            Some(quality) => hasher.update([1u8, quality]),
            None => hasher.update([0u8, 0u8]),
        }
        let digest = hasher.finalize();
        format!("{:02x}{:02x}{:02x}", digest[0], digest[1], digest[2])
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

    /// Deterministic 1200x800 gradient with LCG noise. Flat-colour images
    /// compress to almost nothing at any quality, so size comparisons need
    /// real detail. Saved as PNG so the on-disk source is lossless.
    fn noisy_image(width: u32, height: u32, alpha: bool) -> image::DynamicImage {
        let mut seed: u32 = 0x1234_5678;
        let mut next = move || {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (seed >> 24) as u8
        };
        let mut img = image::RgbaImage::new(width, height);
        for y in 0..height {
            for x in 0..width {
                let r = (x * 255 / width) as u8;
                let g = (y * 255 / height) as u8;
                let b = ((x + y) * 255 / (width + height)) as u8;
                let noise = next() / 8;
                let a = if alpha && x < width / 4 && y < height / 4 {
                    0
                } else {
                    255
                };
                img.put_pixel(
                    x,
                    y,
                    image::Rgba([
                        r.saturating_add(noise),
                        g.saturating_add(noise),
                        b.saturating_add(noise),
                        a,
                    ]),
                );
            }
        }
        if alpha {
            image::DynamicImage::ImageRgba8(img)
        } else {
            image::DynamicImage::ImageRgb8(image::DynamicImage::ImageRgba8(img).to_rgb8())
        }
    }

    fn create_noisy_image(dir: &Path, name: &str, alpha: bool) -> PathBuf {
        let path = dir.join(name);
        noisy_image(1200, 800, alpha).save(&path).unwrap();
        path
    }

    /// Encode `source` at full size (single 1200w variant) and return the
    /// variant path and its bytes.
    fn encode_full(source: &Path, out: &Path, format: &str, quality: u8) -> (PathBuf, Vec<u8>) {
        let config = ImageConfig {
            widths: vec![1200],
            quality,
            format: format.to_string(),
            ..Default::default()
        };
        let processor = ImageProcessor::new(config, out.to_path_buf());
        let result = processor.process(source, "alt").unwrap();
        assert_eq!(result.meta.variants.len(), 1);
        let path = result.meta.variants[0].path.clone();
        let bytes = std::fs::read(&path).unwrap();
        (path, bytes)
    }

    #[test]
    fn test_jpeg_quality_affects_size() {
        let temp = TempDir::new().unwrap();
        let source = create_noisy_image(temp.path(), "hero.png", false);

        let (_, low) = encode_full(&source, &temp.path().join("q40"), "jpeg", 40);
        let (_, high) = encode_full(&source, &temp.path().join("q95"), "jpeg", 95);

        assert!(
            low.len() < high.len(),
            "q40 ({}) should be smaller than q95 ({})",
            low.len(),
            high.len()
        );
        for bytes in [&low, &high] {
            let decoded = image::load_from_memory(bytes).unwrap();
            assert_eq!(decoded.dimensions(), (1200, 800));
        }
    }

    #[cfg(feature = "webp-lossy")]
    #[test]
    fn test_webp_lossy_is_much_smaller_than_lossless() {
        let temp = TempDir::new().unwrap();
        let source = create_noisy_image(temp.path(), "hero.png", false);

        // The pre-fix baseline: the `image` crate's lossless-only WebP encoder.
        let img = image::open(&source).unwrap();
        let mut lossless = std::io::Cursor::new(Vec::new());
        img.write_to(&mut lossless, image::ImageFormat::WebP)
            .unwrap();
        let lossless = lossless.into_inner();

        let (_, lossy) = encode_full(&source, &temp.path().join("dist"), "webp", 80);

        assert!(
            (lossy.len() as f64) <= (lossless.len() as f64) * 0.6,
            "WebP q80 ({}) should be at least 40% smaller than lossless ({})",
            lossy.len(),
            lossless.len()
        );
        for bytes in [&lossy, &lossless] {
            let decoded = image::load_from_memory(bytes).unwrap();
            assert_eq!(decoded.dimensions(), (1200, 800));
        }
    }

    #[cfg(feature = "webp-lossy")]
    #[test]
    fn test_webp_lossy_preserves_alpha() {
        let temp = TempDir::new().unwrap();
        let source = create_noisy_image(temp.path(), "hero.png", true);

        let (_, bytes) = encode_full(&source, &temp.path().join("dist"), "webp", 80);
        let decoded = image::load_from_memory(&bytes).unwrap();

        assert!(
            decoded.color().has_alpha(),
            "decoded WebP should keep alpha"
        );
        let rgba = decoded.to_rgba8();
        assert_eq!(rgba.get_pixel(10, 10)[3], 0, "transparent region lost");
        assert_eq!(rgba.get_pixel(1100, 700)[3], 255, "opaque region changed");
    }

    #[cfg(not(feature = "webp-lossy"))]
    #[test]
    fn test_webp_without_feature_is_lossless_and_ignores_quality() {
        let temp = TempDir::new().unwrap();
        let source = create_noisy_image(temp.path(), "hero.png", false);

        let img = image::open(&source).unwrap();
        let mut lossless = std::io::Cursor::new(Vec::new());
        img.write_to(&mut lossless, image::ImageFormat::WebP)
            .unwrap();

        let (path_a, a) = encode_full(&source, &temp.path().join("dist"), "webp", 10);
        let (path_b, b) = encode_full(&source, &temp.path().join("dist"), "webp", 100);

        assert_eq!(a, lossless.into_inner());
        assert_eq!(a, b);
        assert_eq!(
            path_a, path_b,
            "quality must not affect cache key when ignored"
        );
    }

    #[test]
    fn test_png_ignores_quality() {
        let temp = TempDir::new().unwrap();
        let source = create_noisy_image(temp.path(), "hero.png", false);

        let (path_a, a) = encode_full(&source, &temp.path().join("dist"), "png", 10);
        let (path_b, b) = encode_full(&source, &temp.path().join("dist"), "png", 100);

        assert_eq!(
            a, b,
            "PNG output must be byte-identical regardless of quality"
        );
        assert_eq!(path_a, path_b, "quality must not affect PNG cache key");
    }

    #[test]
    fn test_quality_changes_variant_filename() {
        let temp = TempDir::new().unwrap();
        let source = create_noisy_image(temp.path(), "hero.png", false);
        let out = temp.path().join("dist");

        let (q40, _) = encode_full(&source, &out, "jpeg", 40);
        let (q40_again, _) = encode_full(&source, &out, "jpeg", 40);
        let (q95, _) = encode_full(&source, &out, "jpeg", 95);

        assert_eq!(q40, q40_again, "same quality must give a stable filename");
        assert_ne!(q40, q95, "changing quality must change the filename");

        #[cfg(feature = "webp-lossy")]
        {
            let (w40, _) = encode_full(&source, &out, "webp", 40);
            let (w95, _) = encode_full(&source, &out, "webp", 95);
            assert_ne!(w40, w95);
        }
    }

    #[test]
    fn test_cache_hit_skips_reencode_when_quality_unchanged() {
        let temp = TempDir::new().unwrap();
        let source = create_noisy_image(temp.path(), "hero.png", false);
        let out = temp.path().join("dist");

        let (path, _) = encode_full(&source, &out, "jpeg", 80);
        // Poison the cached variant; a re-encode would overwrite it.
        std::fs::write(&path, b"cached").unwrap();

        let (path_again, bytes) = encode_full(&source, &out, "jpeg", 80);
        assert_eq!(path, path_again);
        assert_eq!(bytes, b"cached", "unchanged quality must hit the cache");
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

    /// The cache key depends on the bytes and the quality, not on where
    /// the file lives or when it was written.
    #[test]
    fn test_cache_key_is_content_based() {
        let temp = TempDir::new().unwrap();
        let output_dir = temp.path().join("dist");
        let processor = ImageProcessor::new(default_config(), output_dir);

        let original = create_test_image(temp.path(), "hero.jpg", 1600, 900);
        let elsewhere = temp.path().join("moved").join("renamed.jpg");
        std::fs::create_dir_all(elsewhere.parent().unwrap()).unwrap();
        std::fs::copy(&original, &elsewhere).unwrap();
        // A copy has its own mtime; make the difference unmistakable.
        // Setting the time needs a writable handle: on Windows a read-only
        // `File::open` is refused with "Access is denied".
        let old = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_000_000);
        std::fs::OpenOptions::new()
            .write(true)
            .open(&elsewhere)
            .unwrap()
            .set_modified(old)
            .unwrap();
        assert_eq!(
            processor.compute_hash(&original),
            processor.compute_hash(&elsewhere),
            "same bytes, different path and mtime: same key"
        );

        let different = create_test_image(temp.path(), "other.jpg", 1600, 901);
        assert_ne!(
            processor.compute_hash(&original),
            processor.compute_hash(&different),
            "different bytes: different key"
        );

        let key = processor.compute_hash(&original);
        assert_eq!(key.len(), 6);
        assert!(key.bytes().all(|b| b.is_ascii_hexdigit()), "{key}");
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
