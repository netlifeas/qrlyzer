use std::collections::HashSet;
use std::io::{Read, Seek};

use fast_image_resize as fr;
use image::GrayImage;
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
#[pyo3(signature = (path, auto_resize=false, *, jpeg_fast_path=false))]
pub fn detect_and_decode(
    py: Python,
    path: &str,
    auto_resize: bool,
    jpeg_fast_path: bool,
) -> PyResult<Vec<String>> {
    py.detach(move || {
        Ok(detect_file(path, auto_resize, jpeg_fast_path)?
            .into_iter()
            .map(|detection| detection.content)
            .collect())
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
        Ok(do_detect_and_decode(image, auto_resize).unwrap_or_default())
    })
}

/// Scan QR codes from an image path and return decoded text with a bbox `(x, y, width, height)`.
#[pyfunction]
#[pyo3(signature = (path, auto_resize=false, *, jpeg_fast_path=false))]
pub fn detect_and_decode_with_bbox(
    py: Python,
    path: &str,
    auto_resize: bool,
    jpeg_fast_path: bool,
) -> PyResult<Vec<DecodedWithBoundingBox>> {
    py.detach(move || {
        Ok(detect_file(path, auto_resize, jpeg_fast_path)?
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
        Ok(do_detect_and_decode_with_bbox(image, auto_resize)
            .unwrap_or_default()
            .into_iter()
            .map(|detection| (detection.content, detection.bbox))
            .collect())
    })
}

fn do_detect_and_decode(image: GrayImage, auto_resize: bool) -> Option<Vec<String>> {
    do_detect_and_decode_with_bbox(image, auto_resize).map(|detections| {
        detections
            .into_iter()
            .map(|detection| detection.content)
            .collect()
    })
}

fn do_detect_and_decode_with_bbox(image: GrayImage, auto_resize: bool) -> Option<Vec<Detection>> {
    let mut decoded: Vec<Detection> = Vec::new();
    let original_width = image.width();
    let original_height = image.height();

    if auto_resize {
        // Determine scaling factor range based on image dimensions.
        let max_dimension = original_width.max(original_height);
        let min_scale = MIN_TARGET_DIMENSION / max_dimension as f32;
        let max_scale = MAX_TARGET_DIMENSION / max_dimension as f32;

        // Iterate through the scaling steps (reverse order for efficiency).
        for scale in (0..=RESIZE_SCALE_STEPS).rev().map(|step| {
            min_scale + (max_scale - min_scale) * step as f32 / RESIZE_SCALE_STEPS as f32
        }) {
            if scale >= 1.0 {
                break;
            }
            let resized = resize_image(&image, scale);
            if let Some(resized) = resized {
                let thresholded = apply_threshold(&resized);
                let rqrr_result = scale_detections_to_original(
                    with_rqrr_with_bbox(thresholded),
                    scale,
                    original_width,
                    original_height,
                );
                try_return!(decoded, rqrr_result);
                let rxing_result = scale_detections_to_original(
                    with_rxing_with_bbox(resized),
                    scale,
                    original_width,
                    original_height,
                );
                try_return!(decoded, rxing_result);
            }
        }
    }
    let thresholded = apply_threshold(&image);
    try_return!(decoded, with_rqrr_with_bbox(thresholded));
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

fn with_rxing_with_bbox(image: GrayImage) -> Vec<Detection> {
    // Uses the rxing library, with a 'TryHarder' hint, for QR code detection.
    let mut result = Vec::new();
    let image_width = image.width();
    let image_height = image.height();
    let mut dch = DecodeHints {
        PossibleFormats: Some(HashSet::from([BarcodeFormat::QR_CODE])),
        TryHarder: Some(true),
        ..Default::default()
    };
    let decode_result = rxing::helpers::detect_multiple_in_luma_with_hints(
        image.into_vec(),
        image_width,
        image_height,
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
        let Some(bbox) = bbox_from_points(&points, image_width, image_height) else {
            continue;
        };
        result.push(Detection {
            content: qr.getText().to_string(),
            bbox,
        });
    }
    result
}

fn image_from_bytes(data: Vec<u8>, width: u32, height: u32) -> PyResult<GrayImage> {
    if data.len() != (width as usize * height as usize) {
        return PyResult::Err(PyValueError::new_err(
            "Data length does not match width and height",
        ));
    }
    let image_result = GrayImage::from_raw(width, height, data);
    let image = match image_result {
        Some(image) => image,
        None => return PyResult::Err(PyValueError::new_err("Could not create image")),
    };
    Ok(image)
}

fn detect_file(path: &str, auto_resize: bool, jpeg_fast_path: bool) -> PyResult<Vec<Detection>> {
    let mut reader =
        image::ImageReader::open(path).map_err(|error| PyIOError::new_err(error.to_string()))?;
    if jpeg_fast_path && auto_resize && reader.format() == Some(image::ImageFormat::Jpeg) {
        let mut input = reader.into_inner();
        if let Some((image, original_dimensions)) = load_scaled_jpeg(&mut input) {
            let decoded_dimensions = image.dimensions();
            let mut detections = do_detect_and_decode_with_bbox(image, true).unwrap_or_default();
            if !detections.is_empty() {
                for detection in &mut detections {
                    detection.bbox =
                        map_jpeg_bbox(detection.bbox, decoded_dimensions, original_dimensions);
                }
                return Ok(detections);
            }
        }
        // Unsupported JPEGs, decode failures, and empty scaled scans retain the
        // existing decoder's full-resolution behavior and error reporting.
        input
            .rewind()
            .map_err(|error| PyIOError::new_err(error.to_string()))?;
        reader = image::ImageReader::with_format(input, image::ImageFormat::Jpeg);
    }
    let image = reader
        .decode()
        .map_err(|error| PyIOError::new_err(error.to_string()))?;
    Ok(do_detect_and_decode_with_bbox(image.into_luma8(), auto_resize).unwrap_or_default())
}

fn load_scaled_jpeg(input: impl Read) -> Option<(GrayImage, (u32, u32))> {
    use jpeg_decoder::{Decoder, PixelFormat};

    let mut decoder = Decoder::new(input);
    if let Some(limit) = image::Limits::default().max_alloc {
        decoder.set_max_decoding_buffer_size(usize::try_from(limit).unwrap_or(usize::MAX));
    }
    decoder.read_info().ok()?;
    let info = decoder.info()?;
    if !matches!(info.pixel_format, PixelFormat::L8 | PixelFormat::RGB24) {
        return None;
    }
    let original = (u32::from(info.width), u32::from(info.height));
    let target = MAX_TARGET_DIMENSION as u16;
    let (width, height) = decoder.scale(target, target).ok()?;
    let dimensions = (u32::from(width), u32::from(height));
    if dimensions == original {
        return None;
    }
    let pixels = decoder.decode().ok()?;
    let image = match info.pixel_format {
        PixelFormat::L8 => GrayImage::from_raw(dimensions.0, dimensions.1, pixels)?,
        PixelFormat::RGB24 => image::DynamicImage::ImageRgb8(image::RgbImage::from_raw(
            dimensions.0,
            dimensions.1,
            pixels,
        )?)
        .into_luma8(),
        _ => unreachable!("unsupported pixel formats were excluded before decoding"),
    };
    Some((image, original))
}

fn map_jpeg_bbox(bbox: BoundingBox, decoded: (u32, u32), original: (u32, u32)) -> BoundingBox {
    let (x, y, width, height) = bbox;
    // IDCT rounds each dimension independently. Map edges with exact integer
    // ratios, rounding outwards so the original-coordinate box contains them.
    let left = (u64::from(x) * u64::from(original.0) / u64::from(decoded.0)) as u32;
    let top = (u64::from(y) * u64::from(original.1) / u64::from(decoded.1)) as u32;
    let right = (u64::from(x + width) * u64::from(original.0))
        .div_ceil(u64::from(decoded.0))
        .min(u64::from(original.0)) as u32;
    let bottom = (u64::from(y + height) * u64::from(original.1))
        .div_ceil(u64::from(decoded.1))
        .min(u64::from(original.1)) as u32;
    (left, top, right - left, bottom - top)
}

/// Applies Otsu's thresholding to enhance the image contrast.
fn apply_threshold(image: &GrayImage) -> GrayImage {
    let thresh = otsu_level(image);
    threshold(image, thresh, ThresholdType::Binary)
}

/// Resizes the image based on the target scale and converts it back to a GrayImage.
fn resize_image(image: &GrayImage, target_scale: f32) -> Option<GrayImage> {
    let width = (image.width() as f32 * target_scale) as u32;
    let height = (image.height() as f32 * target_scale) as u32;
    if width == 0 || height == 0 {
        return None;
    }

    let mut dst_image = GrayImage::new(width, height);
    let mut resizer = fr::Resizer::new();
    match resizer.resize(image, &mut dst_image, &fr::ResizeOptions::default()) {
        Ok(_) => Some(dst_image),
        Err(_) => None,
    }
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

#[cfg(test)]
mod tests {
    use super::map_jpeg_bbox;

    #[test]
    fn jpeg_bbox_rounds_outwards_at_odd_dimensions_and_image_edges() {
        let decoded = (1501, 1001);
        let original = (6001, 4003);
        assert_eq!(map_jpeg_bbox((1, 1, 1, 1), decoded, original), (3, 3, 5, 5));
        assert_eq!(
            map_jpeg_bbox((1500, 1000, 1, 1), decoded, original),
            (5997, 3999, 4, 4),
        );
        assert_eq!(
            map_jpeg_bbox((0, 0, 1501, 1001), decoded, original),
            (0, 0, 6001, 4003),
        );
    }
}
