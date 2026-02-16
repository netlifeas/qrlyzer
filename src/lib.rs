use fast_image_resize as fr;
use image::{DynamicImage, GrayImage};
use imageproc::contrast::{otsu_level, threshold, ThresholdType};
use pyo3::{
    exceptions::{PyIOError, PyValueError},
    prelude::*,
};
use rxing::{self, BarcodeFormat, DecodeHints};

/// Minimum target dimension for resizing the image.
const MIN_TARGET_DIMENSION: f32 = 100.0;
/// Maximum target dimension for resizing the image.
const MAX_TARGET_DIMENSION: f32 = 1280.0;
/// Number of scaling steps to apply when resizing the image.
const RESIZE_SCALE_STEPS: u32 = 5;

type BoundingBox = (u32, u32, u32, u32);
type DecodedWithBoundingBox = (String, BoundingBox);

#[derive(Debug, Clone)]
struct Detection {
    content: String,
    bbox: BoundingBox,
}

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
    py.detach(move || {
        let image = load_image(path)?;
        let image = DynamicImage::from(image.into_luma8());
        Ok(do_detect_and_decode(&image, auto_resize).unwrap_or_default())
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
    py.detach(move || {
        let image = image_from_bytes(data, width, height)?;
        Ok(do_detect_and_decode(&image, auto_resize).unwrap_or_default())
    })
}

/// Scan QR codes from an image path and return decoded text with a bbox `(x, y, width, height)`.
#[pyfunction]
#[pyo3(signature = (path, auto_resize=false))]
pub fn detect_and_decode_with_bbox(
    py: Python,
    path: &str,
    auto_resize: bool,
) -> PyResult<Vec<DecodedWithBoundingBox>> {
    py.detach(move || {
        let image = load_image(path)?;
        let image = DynamicImage::from(image.into_luma8());
        Ok(do_detect_and_decode_with_bbox(&image, auto_resize)
            .unwrap_or_default()
            .into_iter()
            .map(|detection| (detection.content, detection.bbox))
            .collect())
    })
}

/// Scan QR codes from grayscale image bytes and return decoded text with a bbox `(x, y, width, height)`.
#[pyfunction]
#[pyo3(signature = (data, width, height, auto_resize=false))]
pub fn detect_and_decode_from_bytes_with_bbox(
    py: Python,
    data: Vec<u8>,
    width: u32,
    height: u32,
    auto_resize: bool,
) -> PyResult<Vec<DecodedWithBoundingBox>> {
    py.detach(move || {
        let image = image_from_bytes(data, width, height)?;
        Ok(do_detect_and_decode_with_bbox(&image, auto_resize)
            .unwrap_or_default()
            .into_iter()
            .map(|detection| (detection.content, detection.bbox))
            .collect())
    })
}

fn do_detect_and_decode(image: &DynamicImage, auto_resize: bool) -> Option<Vec<String>> {
    do_detect_and_decode_with_bbox(image, auto_resize).map(|detections| {
        detections
            .into_iter()
            .map(|detection| detection.content)
            .collect()
    })
}

fn do_detect_and_decode_with_bbox(
    image: &DynamicImage,
    auto_resize: bool,
) -> Option<Vec<Detection>> {
    let mut decoded: Vec<Detection> = Vec::new();
    if auto_resize {
        // Determine scaling factor range based on image dimensions.
        let min_scale = MIN_TARGET_DIMENSION / (image.width().max(image.height())) as f32;
        let max_scale = MAX_TARGET_DIMENSION / (image.width().max(image.height())) as f32;

        // Iterate through the scaling steps (reverse order for efficiency).
        for scale in (0..=RESIZE_SCALE_STEPS).rev().map(|step| {
            min_scale + (max_scale - min_scale) * step as f32 / RESIZE_SCALE_STEPS as f32
        }) {
            if scale >= 1.0 {
                break;
            }
            let resized = resize_image(image, scale);
            if let Some(resized) = resized {
                let thresholded = apply_threshold(&resized);
                let rqrr_result = scale_detections_to_original(
                    with_rqrr_with_bbox(thresholded.into_luma8()),
                    scale,
                    image.width(),
                    image.height(),
                );
                try_return!(decoded, rqrr_result);
                let rxing_result = scale_detections_to_original(
                    with_rxing_with_bbox(&resized),
                    scale,
                    image.width(),
                    image.height(),
                );
                try_return!(decoded, rxing_result);
            }
        }
    }
    let thresholded = apply_threshold(image);
    try_return!(decoded, with_rqrr_with_bbox(thresholded.into_luma8()));
    try_return!(decoded, with_rxing_with_bbox(image));
    Some(decoded)
}

fn with_rqrr_with_bbox(image: GrayImage) -> Vec<Detection> {
    // Uses the rqrr library for QR code detection.
    let mut result = Vec::new();
    let image_width = image.width();
    let image_height = image.height();
    let mut prepared_image = rqrr::PreparedImage::prepare(image);
    let grids = prepared_image.detect_grids();
    for grid in grids.into_iter() {
        // Attempt to decode each detected grid.
        let decode_result = grid.decode();
        let (_meta, content) = match decode_result {
            Ok((meta, content)) => (meta, content),
            Err(_) => continue,
        };
        let points: Vec<(f32, f32)> = grid
            .bounds
            .iter()
            .map(|point| (point.x as f32, point.y as f32))
            .collect();
        let Some(bbox) = bbox_from_points(&points, image_width, image_height) else {
            continue;
        };
        result.push(Detection { content, bbox });
    }
    result
}

fn with_rxing_with_bbox(image: &DynamicImage) -> Vec<Detection> {
    // Uses the rxing library, with a 'TryHarder' hint, for QR code detection.
    let mut result = Vec::new();
    let mut dch = DecodeHints {
        TryHarder: Some(true),
        ..Default::default()
    };
    let decode_result = rxing::helpers::detect_multiple_in_luma_with_hints(
        image.to_luma8().into_vec(),
        image.width(),
        image.height(),
        &mut dch,
    );
    let decoded = match decode_result {
        Ok(result) => result,
        Err(_) => return result,
    };
    for qr in decoded.into_iter() {
        if *qr.getBarcodeFormat() != BarcodeFormat::QR_CODE {
            continue;
        }
        let points: Vec<(f32, f32)> = qr
            .getPoints()
            .iter()
            .map(|point| (point.x, point.y))
            .collect();
        let Some(bbox) = bbox_from_points(&points, image.width(), image.height()) else {
            continue;
        };
        result.push(Detection {
            content: qr.getText().to_string(),
            bbox,
        });
    }
    result
}

fn image_from_bytes(data: Vec<u8>, width: u32, height: u32) -> PyResult<DynamicImage> {
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
    Ok(image)
}

fn load_image(path: &str) -> PyResult<DynamicImage> {
    let image = image::open(path);
    match image {
        Ok(image) => PyResult::Ok(image),
        Err(image_err) => PyResult::Err(PyIOError::new_err(image_err.to_string())),
    }
}

/// Applies Otsu's thresholding to enhance the image contrast.
fn apply_threshold(image: &DynamicImage) -> DynamicImage {
    let luma8 = match image.as_luma8() {
        Some(luma8) => luma8,
        None => {
            // This should never happen, but if it does, return a copy of the original image.
            return image.clone();
        }
    };

    let thresh = otsu_level(luma8);
    DynamicImage::from(threshold(luma8, thresh, ThresholdType::Binary))
}

/// Resizes the image based on the target scale and converts it back to a GrayImage.
fn resize_image(image: &DynamicImage, target_scale: f32) -> Option<DynamicImage> {
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

fn bbox_from_points(
    points: &[(f32, f32)],
    image_width: u32,
    image_height: u32,
) -> Option<BoundingBox> {
    if points.is_empty() {
        return None;
    }

    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;

    for (x, y) in points.iter().copied() {
        if !x.is_finite() || !y.is_finite() {
            continue;
        }
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x);
        max_y = max_y.max(y);
    }

    if !min_x.is_finite() || !min_y.is_finite() || !max_x.is_finite() || !max_y.is_finite() {
        return None;
    }

    let left = clamp_coordinate(min_x.floor(), image_width);
    let top = clamp_coordinate(min_y.floor(), image_height);
    let right = clamp_coordinate(max_x.ceil(), image_width);
    let bottom = clamp_coordinate(max_y.ceil(), image_height);

    Some((
        left,
        top,
        right.saturating_sub(left),
        bottom.saturating_sub(top),
    ))
}

fn scale_detections_to_original(
    detections: Vec<Detection>,
    scale: f32,
    original_width: u32,
    original_height: u32,
) -> Vec<Detection> {
    if detections.is_empty() || scale <= 0.0 {
        return detections;
    }

    detections
        .into_iter()
        .map(|detection| Detection {
            content: detection.content,
            bbox: scale_bbox_to_original(detection.bbox, scale, original_width, original_height),
        })
        .collect()
}

fn scale_bbox_to_original(
    bbox: BoundingBox,
    scale: f32,
    original_width: u32,
    original_height: u32,
) -> BoundingBox {
    let (x, y, width, height) = bbox;
    let right = x.saturating_add(width);
    let bottom = y.saturating_add(height);

    let left_scaled = clamp_coordinate((x as f32 / scale).floor(), original_width);
    let top_scaled = clamp_coordinate((y as f32 / scale).floor(), original_height);
    let right_scaled = clamp_coordinate((right as f32 / scale).ceil(), original_width);
    let bottom_scaled = clamp_coordinate((bottom as f32 / scale).ceil(), original_height);

    (
        left_scaled,
        top_scaled,
        right_scaled.saturating_sub(left_scaled),
        bottom_scaled.saturating_sub(top_scaled),
    )
}

fn clamp_coordinate(value: f32, max: u32) -> u32 {
    if !value.is_finite() {
        return 0;
    }
    value.clamp(0.0, max as f32) as u32
}

/// qrlyzer QR code reader module.
#[pymodule(gil_used = false)]
fn qrlyzer(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(detect_and_decode, m)?)?;
    m.add_function(wrap_pyfunction!(detect_and_decode_from_bytes, m)?)?;
    m.add_function(wrap_pyfunction!(detect_and_decode_with_bbox, m)?)?;
    m.add_function(wrap_pyfunction!(detect_and_decode_from_bytes_with_bbox, m)?)?;
    Ok(())
}
