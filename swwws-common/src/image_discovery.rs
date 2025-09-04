use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use crate::error::{SwwwsError, ImageDiscoveryError};
use crate::Result;

pub struct ImageDiscovery;

impl ImageDiscovery {
    pub fn discover_images(path: &Path) -> Result<Vec<PathBuf>> {
        if !path.exists() {
            return Err(SwwwsError::ImageDiscovery(ImageDiscoveryError::DirectoryRead {
                path: path.to_path_buf(),
                source: std::io::Error::new(std::io::ErrorKind::NotFound, "Directory not found"),
            }));
        }

        if !path.is_dir() {
            return Err(SwwwsError::ImageDiscovery(ImageDiscoveryError::DirectoryRead {
                path: path.to_path_buf(),
                source: std::io::Error::new(std::io::ErrorKind::InvalidInput, "Path is not a directory"),
            }));
        }

        let mut images = Vec::new();
        let supported_extensions = ["jpg", "jpeg", "png", "gif", "bmp", "tiff", "webp", "avif"];

        for entry in WalkDir::new(path)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let entry_path = entry.path();
            
            if entry_path.is_file() {
                if let Some(extension) = entry_path.extension() {
                    if let Some(ext_str) = extension.to_str() {
                        if supported_extensions.contains(&ext_str.to_lowercase().as_str()) {
                            // Validate that the file is actually readable
                            if let Err(e) = std::fs::metadata(entry_path) {
                                log::warn!("Skipping unreadable file {:?}: {}", entry_path, e);
                                continue;
                            }
                            
                            images.push(entry_path.to_path_buf());
                        }
                    }
                }
            }
        }

        if images.is_empty() {
            return Err(SwwwsError::ImageDiscovery(ImageDiscoveryError::NoImagesFound {
                path: path.to_path_buf(),
            }));
        }

        log::info!("Discovered {} images in {:?}", images.len(), path);
        Ok(images)
    }

    pub fn validate_image(path: &Path) -> Result<()> {
        if !path.exists() {
            return Err(SwwwsError::ImageDiscovery(ImageDiscoveryError::FileAccess {
                path: path.to_path_buf(),
                source: std::io::Error::new(std::io::ErrorKind::NotFound, "File not found"),
            }));
        }

        if !path.is_file() {
            return Err(SwwwsError::ImageDiscovery(ImageDiscoveryError::FileAccess {
                path: path.to_path_buf(),
                source: std::io::Error::new(std::io::ErrorKind::InvalidInput, "Not a file"),
            }));
        }

        // First check by extension for basic validation
        let extension = path.extension()
            .and_then(|ext| ext.to_str())
            .map(|s| s.to_lowercase());

        match extension.as_deref() {
            Some("jpg") | Some("jpeg") | Some("png") | Some("gif") |
            Some("bmp") | Some("tiff") | Some("webp") | Some("avif") => {},
            _ => return Err(SwwwsError::ImageDiscovery(ImageDiscoveryError::UnsupportedFormat {
                path: path.to_path_buf(),
            })),
        }

        // Then validate by reading file header
        Self::validate_image_header(path)
    }

    fn validate_image_header(path: &Path) -> Result<()> {
        use std::fs::File;
        use std::io::Read;

        let mut file = File::open(path)
            .map_err(|e| SwwwsError::ImageDiscovery(ImageDiscoveryError::FileAccess {
                path: path.to_path_buf(),
                source: e,
            }))?;

        let mut header = [0u8; 12];
        match file.read(&mut header) {
            Ok(bytes_read) if bytes_read >= 4 => {
                // Check magic bytes for common image formats
                match &header[0..4] {
                    [0xFF, 0xD8, 0xFF, _] => Ok(()), // JPEG
                    [0x89, 0x50, 0x4E, 0x47] => Ok(()), // PNG
                    [0x47, 0x49, 0x46, 0x38] => Ok(()), // GIF
                    [0x42, 0x4D, _, _] => Ok(()), // BMP
                    [0x52, 0x49, 0x46, 0x46] if bytes_read >= 12 && &header[8..12] == b"WEBP" => Ok(()), // WebP
                    _ => {
                        // For TIFF and AVIF, check more bytes if needed
                        if bytes_read >= 8 {
                            match &header[0..8] {
                                [0x49, 0x49, 0x2A, 0x00, _, _, _, _] => Ok(()), // TIFF little endian
                                [0x4D, 0x4D, 0x00, 0x2A, _, _, _, _] => Ok(()), // TIFF big endian
                                _ => Err(SwwwsError::ImageDiscovery(ImageDiscoveryError::CorruptedImage {
                                    path: path.to_path_buf(),
                                })),
                            }
                        } else {
                            Err(SwwwsError::ImageDiscovery(ImageDiscoveryError::CorruptedImage {
                                path: path.to_path_buf(),
                            }))
                        }
                    }
                }
            }
            _ => Err(SwwwsError::ImageDiscovery(ImageDiscoveryError::CorruptedImage {
                path: path.to_path_buf(),
            }))
        }
    }

    pub fn get_supported_extensions() -> Vec<&'static str> {
        vec!["jpg", "jpeg", "png", "gif", "bmp", "tiff", "webp", "avif"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs;
    use std::path::Path;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn test_discover_images_success() {
        let temp_dir = tempdir().unwrap();
        let test_dir = temp_dir.path();
        
        // Create test images
        fs::write(test_dir.join("image1.jpg"), "fake jpg").unwrap();
        fs::write(test_dir.join("image2.png"), "fake png").unwrap();
        fs::write(test_dir.join("image3.gif"), "fake gif").unwrap();
        fs::write(test_dir.join("text.txt"), "not an image").unwrap();
        
        let images = ImageDiscovery::discover_images(test_dir).unwrap();
        
        assert_eq!(images.len(), 3);
        assert!(images.iter().any(|p| p.file_name().unwrap() == "image1.jpg"));
        assert!(images.iter().any(|p| p.file_name().unwrap() == "image2.png"));
        assert!(images.iter().any(|p| p.file_name().unwrap() == "image3.gif"));
        assert!(!images.iter().any(|p| p.file_name().unwrap() == "text.txt"));
    }

    #[test]
    fn test_discover_images_empty_directory() {
        let temp_dir = tempdir().unwrap();
        let test_dir = temp_dir.path();
        
        let result = ImageDiscovery::discover_images(test_dir);
        assert!(result.is_err());
        
        match result.unwrap_err() {
            SwwwsError::ImageDiscovery(ImageDiscoveryError::NoImagesFound { path }) => {
                assert_eq!(path, test_dir);
            },
            _ => panic!("Expected NoImagesFound error"),
        }
    }

    #[test]
    fn test_discover_images_nonexistent_directory() {
        let nonexistent_path = Path::new("/nonexistent/directory");
        
        let result = ImageDiscovery::discover_images(nonexistent_path);
        assert!(result.is_err());
        
        match result.unwrap_err() {
            SwwwsError::ImageDiscovery(ImageDiscoveryError::DirectoryRead { path, .. }) => {
                assert_eq!(path, nonexistent_path);
            },
            _ => panic!("Expected DirectoryRead error"),
        }
    }

    #[test]
    fn test_validate_image() {
        let temp_dir = tempdir().unwrap();
        let test_dir = temp_dir.path();
        
        // Create a valid image file with proper JPEG header
        let jpeg_header = [0xFF, 0xD8, 0xFF, 0xE0]; // JPEG magic bytes
        fs::write(test_dir.join("valid.jpg"), &jpeg_header).unwrap();
        let valid_path = test_dir.join("valid.jpg");
        
        // Create an invalid file
        fs::write(test_dir.join("invalid.txt"), "not an image").unwrap();
        let invalid_path = test_dir.join("invalid.txt");
        
        // Create a nonexistent file
        let nonexistent_path = test_dir.join("nonexistent.jpg");
        
        assert!(ImageDiscovery::validate_image(&valid_path).is_ok());
        assert!(ImageDiscovery::validate_image(&invalid_path).is_err());
        assert!(ImageDiscovery::validate_image(&nonexistent_path).is_err());
    }

    #[test]
    fn test_get_supported_extensions() {
        let extensions = ImageDiscovery::get_supported_extensions();
        
        assert!(extensions.contains(&"jpg"));
        assert!(extensions.contains(&"jpeg"));
        assert!(extensions.contains(&"png"));
        assert!(extensions.contains(&"gif"));
        assert!(extensions.contains(&"bmp"));
        assert!(extensions.contains(&"tiff"));
        assert!(extensions.contains(&"webp"));
        assert!(extensions.contains(&"avif"));
        
        // Should not contain non-image extensions
        assert!(!extensions.contains(&"txt"));
        assert!(!extensions.contains(&"pdf"));
    }

    #[test]
    fn test_discover_images_case_insensitive() {
        let temp_dir = tempdir().unwrap();
        let test_dir = temp_dir.path();
        
        // Create test images with different case extensions
        fs::write(test_dir.join("image1.JPG"), "fake jpg").unwrap();
        fs::write(test_dir.join("image2.PNG"), "fake png").unwrap();
        fs::write(test_dir.join("image3.GIF"), "fake gif").unwrap();
        
        let images = ImageDiscovery::discover_images(test_dir).unwrap();
        
        assert_eq!(images.len(), 3);
        assert!(images.iter().any(|p| p.file_name().unwrap() == "image1.JPG"));
        assert!(images.iter().any(|p| p.file_name().unwrap() == "image2.PNG"));
        assert!(images.iter().any(|p| p.file_name().unwrap() == "image3.GIF"));
    }

    #[test]
    fn test_discover_images_subdirectories() {
        let temp_dir = tempdir().unwrap();
        let test_dir = temp_dir.path();
        
        // Create subdirectory
        let subdir = test_dir.join("subdir");
        fs::create_dir(&subdir).unwrap();
        
        // Create images in both root and subdirectory
        fs::write(test_dir.join("root.jpg"), "fake jpg").unwrap();
        fs::write(subdir.join("sub.png"), "fake png").unwrap();
        
        let images = ImageDiscovery::discover_images(test_dir).unwrap();
        
        assert_eq!(images.len(), 2);
        assert!(images.iter().any(|p| p.file_name().unwrap() == "root.jpg"));
        assert!(images.iter().any(|p| p.file_name().unwrap() == "sub.png"));
    }

    #[test]
    fn test_discover_images_permission_error() {
        // Skip this test on non-Unix systems
        #[cfg(not(unix))]
        {
            return;
        }
        
        let temp_dir = tempdir().unwrap();
        let test_dir = temp_dir.path();
        
        // Create a directory we can't read (simulate permission error)
        let restricted_dir = test_dir.join("restricted");
        fs::create_dir(&restricted_dir).unwrap();
        
        // Remove read permissions
        let mut perms = fs::metadata(&restricted_dir).unwrap().permissions();
        perms.set_mode(0o000);
        fs::set_permissions(&restricted_dir, perms).unwrap();
        
        let result = ImageDiscovery::discover_images(&restricted_dir);
        assert!(result.is_err());
        
        // Restore permissions for cleanup
        let mut perms = fs::metadata(&restricted_dir).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&restricted_dir, perms).unwrap();
        
        // Just check that it's an error, don't be specific about the type
        // since the error might be different depending on the system
        assert!(result.is_err());
    }
}
