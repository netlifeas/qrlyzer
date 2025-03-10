use fast_image_resize as fr;
use image::{DynamicImage, GrayImage};
use imageproc::contrast::{otsu_level, threshold, ThresholdType};
use pyo3::{
    exceptions::{PyIOError, PyValueError},
    prelude::*,
};
use rqrr;
use rxing::{self, BarcodeFormat, DecodeHints};

macro_rules! try_return {
    ($decoded:expr, $new:expr) => {{
        $decoded.extend($new);
        if !$decoded.is_empty() {
            return Some($decoded);
        }
    }};
}

/// Scan QR codes from an image given as a path.
#[pyfunction]
#[pyo3(signature = (path, auto_resize=false))]
pub fn detect_and_decode(py: Python, path: &str, auto_resize: bool) -> PyResult<Vec<String>> {
    // Entry point for QR code detection from a file path.
    py.allow_threads(move || {
        let mut decoded: Vec<String> = Vec::new();
        let image = load_image(path)?;
        let image = DynamicImage::from(image.into_luma8());
        if let Some(result) = do_detect_and_decode(&image, auto_resize) {
            decoded.extend(result);
        }
        Ok(decoded)
    })
}

/// Scan QR codes from a grayscale image given in bytes.
#[pyfunction]
#[pyo3(signature = (data, width, height, auto_resize=false))]
pub fn detect_and_decode_from_bytes(
    py: Python,
    data: Vec<u8>,
    width: u32,
    height: u32,
    auto_resize: bool,
) -> PyResult<Vec<String>> {
    // Entry point for QR code detection from raw image bytes.
    py.allow_threads(move || {
        let mut decoded: Vec<String> = Vec::new();
        if data.len() != (width as usize * height as usize) {
            return PyResult::Err(PyValueError::new_err(
                "Data length does not match width and height",
            ));
        }
        let image_result = GrayImage::from_raw(width, height, data);
        let image = match image_result {
            Some(image) => DynamicImage::from(image),
            None => return PyResult::Err(PyValueError::new_err("Could not create image")),
        };
        if let Some(result) = do_detect_and_decode(&image, auto_resize) {
            decoded.extend(result);
        }
        Ok(decoded)
    })
}

fn do_detect_and_decode(image: &DynamicImage, auto_resize: bool) -> Option<Vec<String>> {
    let mut decoded: Vec<String> = Vec::new();
    if auto_resize {
        // Determine scaling factor range based on image dimensions.
        let min_scale = 100.0 / (image.width().max(image.height())) as f32;
        let max_scale = 1280.0 / (image.width().max(image.height())) as f32;
        let scale_steps = 5;

        // Iterate through defined scaling steps (reverse order for efficiency).
        for scale in (0..=scale_steps)
            .rev()
            .map(|step| min_scale + (max_scale - min_scale) * step as f32 / scale_steps as f32)
        {
            if scale >= 1.0 {
                break;
            }
            // Resize image and apply thresholding to enhance QR detection.
            let resized = resize_image(&image, scale);
            if let Some(resized) = resized {
                let thresholded = apply_threshold(&resized);
                try_return!(decoded, with_rqrr(thresholded.into_luma8()));
                try_return!(decoded, with_rxing(&resized));
            }
        }
    }
    // Process non-resized image.
    let thresholded = apply_threshold(&image);
    try_return!(decoded, with_rqrr(thresholded.into_luma8()));
    try_return!(decoded, with_rxing(&image));
    Some(decoded)
}

fn with_rqrr(image: GrayImage) -> Vec<String> {
    // Uses the rqrr library for QR code detection.
    let mut result = Vec::new();
    let mut prepared_image = rqrr::PreparedImage::prepare(image);
    let grids = prepared_image.detect_grids();
    for grid in grids.into_iter() {
        // Attempt to decode each detected grid.
        let decode_result = grid.decode();
        let (_meta, content) = match decode_result {
            Ok((meta, content)) => (meta, content),
            Err(_) => continue,
        };
        result.push(content.to_string());
    }
    result
}

fn with_rxing(image: &DynamicImage) -> Vec<String> {
    // Uses the rxing library, with a 'TryHarder' hint, for QR code detection.
    let mut result = Vec::new();
    let mut dch = DecodeHints {
        TryHarder: Some(true),
        ..Default::default()
    };
    let decode_result = rxing::helpers::detect_in_luma_with_hints(
        image.to_luma8().into_vec(),
        image.width(),
        image.height(),
        Some(BarcodeFormat::QR_CODE),
        &mut dch,
    );
    let decoded = match decode_result {
        Ok(result) => result,
        Err(_) => return result,
    };
    result.push(decoded.getText().to_string());
    result
}

fn load_image(path: &str) -> PyResult<DynamicImage> {
    // Loads an image from a given path and converts it to grayscale.
    let image = image::open(path);
    match image {
        Ok(image) => PyResult::Ok(image),
        Err(_) => return PyResult::Err(PyIOError::new_err("Could not load image")),
    }
}

fn apply_threshold(image: &DynamicImage) -> DynamicImage {
    // Applies Otsu's thresholding to enhance the image contrast.

    let luma8 = match image.as_luma8() {
        Some(luma8) => luma8,
        None => {
            // This should never happen, but if it does, return a copy of the original image.
            return image.clone();
        }
    };

    let thresh = otsu_level(&luma8);
    DynamicImage::from(threshold(&luma8, thresh, ThresholdType::Binary))
}

fn resize_image(image: &DynamicImage, target_scale: f32) -> Option<DynamicImage> {
    // Resizes the image based on the target scale and converts it back to a GrayImage.
    let mut dst_image = DynamicImage::new_luma8(
        (image.width() as f32 * target_scale) as u32,
        (image.height() as f32 * target_scale) as u32,
    );

    let mut resizer = fr::Resizer::new();
    let dst_image = match resizer.resize(image, &mut dst_image, &fr::ResizeOptions::default()) {
        Ok(_) => dst_image,
        Err(_) => return None,
    };
    Some(dst_image)
}

/// qrlyzer QR code reader module.
#[pymodule(gil_used = false)]
fn qrlyzer(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(detect_and_decode, m)?)?;
    m.add_function(wrap_pyfunction!(detect_and_decode_from_bytes, m)?)?;
    Ok(())
}
