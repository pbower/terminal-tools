//! Native image preview functionality for terminal display.
//!
//! This module provides image preview capabilities for terminal tools without requiring
//! external dependencies. Images are converted to ASCII art for display in text interfaces.
//!
//! ## Features
//!
//! - **Format Support**: JPG, PNG, GIF, BMP (conservative format selection for stability)
//! - **ASCII Art Generation**: Converts images to text representation using grayscale mapping
//! - **Safety Checks**: Handles large, corrupted, or invalid images gracefully
//! - **Performance**: Optimized for terminal display with reasonable size limits
//!
//! ## Usage
//!
//! The module is primarily used by file exploration tools to provide image previews:
//!
//! ```rust
//! use std::path::Path;
//! use crate::image_preview::{is_image_file, generate_image_preview};
//!
//! let path = Path::new("image.jpg");
//! if is_image_file(path) {
//!     let preview = generate_image_preview(path);
//!     println!("{}", preview);
//! }
//! ```
//!
//! ## Safety & Error Handling
//!
//! The module includes comprehensive safety measures:
//! - Panic recovery using `std::panic::catch_unwind`
//! - Dimension validation (rejects zero or excessive dimensions)
//! - Bounds checking for all pixel access operations
//! - Conservative format support to avoid compatibility issues
//!
//! ## ASCII Art Generation
//!
//! Images are converted to ASCII art using:
//! 1. Resize to terminal-appropriate dimensions (40x15)
//! 2. Convert to grayscale using standard RGB weights
//! 3. Map grayscale values to ASCII characters (" .:-=+*#%@")
//! 4. Generate text representation suitable for terminal display

use std::path::Path;
use image::GenericImageView;

/// Check if a file is a supported image format
pub fn is_image_file(path: &Path) -> bool {
    if let Some(extension) = path.extension() {
        let ext = extension.to_string_lossy().to_lowercase();
        // Be more conservative with supported formats to avoid issues
        matches!(ext.as_str(), "jpg" | "jpeg" | "png" | "gif" | "bmp")
    } else {
        false
    }
}

/// Generate image preview text for terminal display
pub fn generate_image_preview(path: &Path) -> String {
    // Add a panic handler to catch any issues
    std::panic::catch_unwind(|| {
        // Try to get image metadata first
        match image::open(path) {
            Ok(img) => {
                let (width, height) = img.dimensions();
                
                // Additional safety check for very large images
                if width > 50000 || height > 50000 {
                    return format!(
                        "🖼️ Image: {}\n❌ Image too large for preview ({}x{})",
                        path.file_name().unwrap_or_default().to_string_lossy(),
                        width, height
                    );
                }
                
                let format = img.color().channel_count();
                
                let mut preview = format!(
                    "🖼️ Image: {}\n",
                    path.file_name().unwrap_or_default().to_string_lossy()
                );
                preview.push_str(&format!("📐 Dimensions: {}x{}\n", width, height));
                preview.push_str(&format!("🎨 Channels: {}\n", format));
                
                // Try to render a small terminal preview, but don't fail the whole preview if it doesn't work
                match render_image_to_terminal(path) {
                    Ok(terminal_preview) => {
                        if !terminal_preview.trim().is_empty() {
                            preview.push_str("\n📺 Terminal Preview:\n");
                            preview.push_str(&terminal_preview);
                        }
                    }
                    Err(_) => {
                        // Just skip the preview if it fails - show basic info only
                        preview.push_str("\n📺 ASCII preview unavailable for this image");
                    }
                }
                
                preview
            }
            Err(e) => {
                format!(
                    "🖼️ Image file: {}\n❌ Error loading image: {}",
                    path.file_name().unwrap_or_default().to_string_lossy(),
                    e
                )
            }
        }
    }).unwrap_or_else(|_| {
        format!(
            "🖼️ Image file: {}\n❌ Panic occurred during image processing",
            path.file_name().unwrap_or_default().to_string_lossy()
        )
    })
}

/// Render image to terminal using viuer
fn render_image_to_terminal(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    use viuer::Config;
    
    // Create a configuration for small terminal preview
    let _config = Config {
        // Make it small to fit in preview pane
        width: Some(40),
        height: Some(20),
        absolute_offset: false,
        ..Default::default()
    };
    
    // Capture the viuer output
    // Note: viuer prints directly to terminal, so we'll return a placeholder
    // In a real implementation, we'd need to capture the ANSI output
    match image::open(path) {
        Ok(img) => {
            let (_width, _height) = img.dimensions();
            
            // Validate image dimensions before processing
            if _width == 0 || _height == 0 {
                return Err("Image has zero dimensions".into());
            }
            
            if _width > 10000 || _height > 10000 {
                return Err("Image too large for preview".into());
            }
            
            // For now, return ASCII art representation
            generate_ascii_preview(&img, 40, 15)
        }
        Err(e) => Err(e.into()),
    }
}

/// Generate simple ASCII art preview
fn generate_ascii_preview(
    img: &image::DynamicImage, 
    target_width: u32, 
    target_height: u32
) -> Result<String, Box<dyn std::error::Error>> {
    use image::imageops::FilterType;
    
    // Ensure reasonable dimensions to prevent issues
    let safe_width = target_width.min(200).max(1);
    let safe_height = target_height.min(100).max(1);
    
    // Resize image to target dimensions
    let resized = img.resize(safe_width, safe_height, FilterType::Nearest);
    let rgb_img = resized.to_rgb8();
    
    let mut ascii_art = String::new();
    
    // ASCII characters from dark to light
    let chars = " .:-=+*#%@";
    let char_vec: Vec<char> = chars.chars().collect();
    
    // Get actual dimensions of the resized image
    let (actual_width, actual_height) = rgb_img.dimensions();
    
    for y in 0..actual_height {
        for x in 0..actual_width {
            // Safely get pixel with bounds checking
            if x < actual_width && y < actual_height {
                let pixel = rgb_img.get_pixel(x, y);
                let [r, g, b] = pixel.0;
                
                // Convert to grayscale
                let gray = (0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32) as u8;
                
                // Map to ASCII character with safe indexing
                let char_index = if gray == 255 {
                    char_vec.len() - 1
                } else {
                    ((gray as usize) * (char_vec.len() - 1)) / 255
                };
                
                let char_index = char_index.min(char_vec.len() - 1);
                ascii_art.push(char_vec[char_index]);
            } else {
                ascii_art.push(' ');
            }
        }
        ascii_art.push('\n');
    }
    
    Ok(ascii_art)
}

/// Get image info without rendering
#[allow(dead_code)]
pub fn get_image_info(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let img = image::open(path)?;
    let (width, height) = img.dimensions();
    
    Ok(format!(
        "🖼️  Image: {}\n📐 Size: {}x{}\n🎨 Format: {:?}",
        path.file_name().unwrap_or_default().to_string_lossy(),
        width,
        height,
        img.color()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_is_image_file_supported_formats() {
        assert!(is_image_file(Path::new("test.jpg")));
        assert!(is_image_file(Path::new("test.jpeg")));
        assert!(is_image_file(Path::new("test.png")));
        assert!(is_image_file(Path::new("test.gif")));
        assert!(is_image_file(Path::new("test.bmp")));
        assert!(is_image_file(Path::new("TEST.JPG"))); // case insensitive
    }

    #[test]
    fn test_is_image_file_unsupported_formats() {
        assert!(!is_image_file(Path::new("test.txt")));
        assert!(!is_image_file(Path::new("test.rs")));
        assert!(!is_image_file(Path::new("test.pdf")));
        assert!(!is_image_file(Path::new("test")));
        assert!(!is_image_file(Path::new("test.")));
    }

    #[test]
    fn test_is_image_file_no_extension() {
        assert!(!is_image_file(Path::new("noextension")));
        assert!(!is_image_file(Path::new(".")));
        assert!(!is_image_file(Path::new("")));
    }

    #[test]
    fn test_generate_image_preview_nonexistent_file() {
        let preview = generate_image_preview(Path::new("nonexistent.jpg"));
        assert!(preview.contains("🖼️ Image"));
        assert!(preview.contains("nonexistent.jpg"));
        assert!(preview.contains("Error loading image") || preview.contains("Panic occurred"));
    }

    #[test]
    fn test_generate_ascii_preview_bounds() {
        // Test that the function handles edge cases safely
        use image::{DynamicImage, RgbImage};
        
        // Create a minimal 1x1 test image
        let img = DynamicImage::ImageRgb8(RgbImage::new(1, 1));
        
        // Test with valid dimensions
        let result = generate_ascii_preview(&img, 10, 5);
        assert!(result.is_ok());
        
        // Test with zero dimensions should be clamped to 1
        let result = generate_ascii_preview(&img, 0, 0);
        assert!(result.is_ok());
        
        // Test with very large dimensions should be clamped
        let result = generate_ascii_preview(&img, 1000, 1000);
        assert!(result.is_ok());
    }
}