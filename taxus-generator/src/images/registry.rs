use super::processor::ProcessedImage;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Default)]
pub struct ImageRegistry {
    images: HashMap<PathBuf, ProcessedImage>,
}

impl ImageRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, source_path: PathBuf, processed: ProcessedImage) {
        self.images.insert(source_path, processed);
    }

    pub fn get(&self, source_path: &PathBuf) -> Option<&ProcessedImage> {
        self.images.get(source_path)
    }

    pub fn len(&self) -> usize {
        self.images.len()
    }

    pub fn is_empty(&self) -> bool {
        self.images.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ImageConfig;
    use crate::images::processor::ImageMeta;
    use std::path::{Path, PathBuf};
    use tempfile::TempDir;

    fn create_test_image(dir: &Path, name: &str, width: u32, height: u32) -> PathBuf {
        let path = dir.join(name);
        let img = image::RgbImage::from_pixel(width, height, image::Rgb([128, 128, 128]));
        img.save(&path).unwrap();
        path
    }

    #[test]
    fn test_registry_insert_and_get() {
        let temp = TempDir::new().unwrap();
        let source = create_test_image(temp.path(), "hero.jpg", 1600, 900);
        let output_dir = temp.path().join("dist");

        let config = ImageConfig::default();
        let processor = crate::images::ImageProcessor::new(config, output_dir);
        let processed = processor.process(&source, "Test").unwrap();

        let mut registry = ImageRegistry::new();
        let key = source.clone();
        registry.insert(key.clone(), processed);

        assert!(registry.get(&key).is_some());
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn test_registry_missing_key() {
        let registry = ImageRegistry::new();
        assert!(registry.get(&PathBuf::from("nonexistent.jpg")).is_none());
        assert!(registry.is_empty());
    }

    #[test]
    fn test_registry_overwrite() {
        let mut registry = ImageRegistry::new();
        let key = PathBuf::from("hero.jpg");

        let processed1 = ProcessedImage {
            source_path: key.clone(),
            output_dir: PathBuf::from("dist"),
            meta: ImageMeta {
                original_width: 100,
                original_height: 100,
                aspect_ratio: 1.0,
                alt: "v1".to_string(),
                variants: vec![],
            },
            format: "webp".to_string(),
        };

        let processed2 = ProcessedImage {
            source_path: key.clone(),
            output_dir: PathBuf::from("dist"),
            meta: ImageMeta {
                original_width: 200,
                original_height: 200,
                aspect_ratio: 1.0,
                alt: "v2".to_string(),
                variants: vec![],
            },
            format: "webp".to_string(),
        };

        registry.insert(key.clone(), processed1);
        registry.insert(key.clone(), processed2);

        assert_eq!(registry.len(), 1);
        assert_eq!(registry.get(&key).unwrap().meta.original_width, 200);
    }
}
