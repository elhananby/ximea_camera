use anyhow::{anyhow, Result};
use xiapi;

/// Represents a single frame captured from the XIMEA camera
#[derive(Clone)]
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
    pub timestamp: u64,
    pub frame_number: u32,
}

impl Frame {
    /// Create a new Frame from a xiapi::Image
    pub fn from_xi_image(image: &xiapi::Image<u8>) -> Result<Self> {
        Ok(Frame {
            width: image.width(),
            height: image.height(),
            data: image.data().to_vec(),
            timestamp: image.timestamp_raw(),
            frame_number: image.nframe(),
        })
    }

    /// Get the total number of pixels in the frame
    pub fn pixel_count(&self) -> u32 {
        self.width * self.height
    }

    /// Get a reference to the raw image data
    pub fn raw_data(&self) -> &[u8] {
        &self.data
    }

    /// Get a mutable reference to the raw image data
    pub fn raw_data_mut(&mut self) -> &mut [u8] {
        &mut self.data
    }

    /// Get the value of a specific pixel
    pub fn get_pixel(&self, x: u32, y: u32) -> Option<u8> {
        if x < self.width && y < self.height {
            let index = (y * self.width + x) as usize;
            self.data.get(index).cloned()
        } else {
            None
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    /// Set the value of a specific pixel
    pub fn set_pixel(&mut self, x: u32, y: u32, value: u8) -> Result<()> {
        if x < self.width && y < self.height {
            let index = (y * self.width + x) as usize;
            if let Some(pixel) = self.data.get_mut(index) {
                *pixel = value;
                Ok(())
            } else {
                Err(anyhow!("Failed to set pixel value: index out of bounds"))
            }
        } else {
            Err(anyhow!("Coordinates out of bounds"))
        }
    }

    /// Convert the frame to a different image format (example: to RGB)
    pub fn to_rgb(&self) -> Result<Vec<u8>> {
        // This is a placeholder implementation. In a real-world scenario,
        // you'd implement proper color conversion based on the camera's
        // color filter array (CFA) pattern and other factors.
        let mut rgb_data = Vec::with_capacity(self.data.len() * 3);
        for &pixel in &self.data {
            rgb_data.extend_from_slice(&[pixel, pixel, pixel]);
        }
        Ok(rgb_data)
    }

    /// Save the frame as a PNG image
    #[cfg(feature = "image")]
    pub fn save_as_png(&self, path: &str) -> Result<()> {
        use image::{ImageBuffer, Luma};

        let img = ImageBuffer::<Luma<u8>, _>::from_raw(self.width, self.height, self.data.clone())
            .ok_or_else(|| anyhow!("Failed to create image buffer"))?;

        img.save(path)
            .map_err(|e| anyhow!("Failed to save image: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_creation() {
        let width = 640;
        let height = 480;
        let data = vec![0; (width * height) as usize];
        let frame = Frame {
            width,
            height,
            data: data.clone(),
            timestamp: 0,
            frame_number: 0,
        };

        assert_eq!(frame.width, width);
        assert_eq!(frame.height, height);
        assert_eq!(frame.data, data);
    }

    #[test]
    fn test_pixel_access() {
        let mut frame = Frame {
            width: 2,
            height: 2,
            data: vec![0, 1, 2, 3],
            timestamp: 0,
            frame_number: 0,
        };

        assert_eq!(frame.get_pixel(0, 0), Some(0));
        assert_eq!(frame.get_pixel(1, 0), Some(1));
        assert_eq!(frame.get_pixel(0, 1), Some(2));
        assert_eq!(frame.get_pixel(1, 1), Some(3));
        assert_eq!(frame.get_pixel(2, 2), None);

        frame.set_pixel(0, 0, 5).unwrap();
        assert_eq!(frame.get_pixel(0, 0), Some(5));
    }

    #[test]
    fn test_to_rgb() {
        let frame = Frame {
            width: 2,
            height: 2,
            data: vec![0, 64, 128, 255],
            timestamp: 0,
            frame_number: 0,
        };

        let rgb = frame.to_rgb().unwrap();
        assert_eq!(rgb, vec![0, 0, 0, 64, 64, 64, 128, 128, 128, 255, 255, 255]);
    }
}
