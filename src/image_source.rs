use anyhow::{anyhow, Result};
use opencv::{
    prelude::*,
    videoio,
    imgcodecs,
    core,
};

use std::path::Path;

/// Image source types for your app: preselected images (by filename in assets),
/// user-supplied file path, camera snapshot, or a placeholder for AI-generated.
pub enum ImageSource {
    Preselected(String), // filename relative to assets/images/
    UserPath(String),    // arbitrary filesystem path
    CameraIndex(i32),    // camera index (0 default)
    AIGenerated(String), // placeholder: prompt or identifier
}

/// Return an OpenCV Mat loaded from the desired source.
/// For CameraIndex, this captures one frame and returns it.
/// For AIGenerated, this is currently a placeholder that returns an error until implemented.
pub fn load_image_from_source(src: &ImageSource) -> Result<Mat> {
    match src {
        ImageSource::Preselected(name) => {
            // Compose path to /assets/images/
            let p = Path::new("assets").join("images").join(name);
            if !p.exists() {
                return Err(anyhow!("Preselected image not found: {}", p.display()));
            }
            let img = imgcodecs::imread(p.to_str().unwrap(), imgcodecs::IMREAD_COLOR)?;
            if img.empty() {
                return Err(anyhow!("Failed to load preselected image: {}", p.display()));
            }
            Ok(img)
        }

        ImageSource::UserPath(path) => {
            let p = Path::new(path);
            if !p.exists() {
                return Err(anyhow!("User image path not found: {}", p.display()));
            }
            let img = imgcodecs::imread(p.to_str().unwrap(), imgcodecs::IMREAD_COLOR)?;
            if img.empty() {
                return Err(anyhow!("Failed to load user image: {}", p.display()));
            }
            Ok(img)
        }

        ImageSource::CameraIndex(idx) => {
            // Open camera, read one frame, then release it
            let mut cam = videoio::VideoCapture::new(*idx, videoio::CAP_ANY)?;
            // Wait a moment for camera to warm up if needed
            if !videoio::VideoCapture::is_opened(&cam)? {
                return Err(anyhow!("Failed to open camera index {}", idx));
            }

            // Grab a few frames to stabilize (optional)
            let mut frame = Mat::default();
            for _ in 0..5 {
                cam.read(&mut frame)?;
            }
            if frame.empty() {
                return Err(anyhow!("Captured frame is empty"));
            }
            Ok(frame)
        }

        ImageSource::AIGenerated(_prompt) => {
            // Placeholder: implement your AI image generation hook (API or local model) here.
            Err(anyhow!("AI-generation source not implemented yet"))
        }
    }
}
