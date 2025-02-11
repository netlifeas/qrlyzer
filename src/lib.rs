use image::{DynamicImage, GrayImage, ImageBuffer, Luma};
use imageproc::contrast::{ThresholdType, otsu_level, threshold};
use pyo3::{
    exceptions::{PyIOError, PyValueError},
    prelude::*,
};
use rqrr;
use rxing::{self, BarcodeFormat, DecodeHints};

fn load_image(path: &str) -> PyResult<image::DynamicImage> {
    let image = image::open(path);
    match image {
        Ok(image) => Ok(image),
        Err(_) => return PyResult::Err(PyIOError::new_err("Could not load image")),
    }
}

fn apply_threshold(image: &DynamicImage) -> GrayImage {
    let luma8 = image.to_luma8();
    let ol = otsu_level(&luma8);
    threshold(&luma8, ol, ThresholdType::Binary)
}

/// Scan QR codes from an image given as a path.
#[pyfunction]
fn detect_and_decode(py: Python, path: &str) -> PyResult<Vec<String>> {
    py.allow_threads(move || {
        let mut decoded: Vec<String> = Vec::new();
        let image = load_image(path)?;
        let luma8_otsu = apply_threshold(&image);
        decoded.extend(with_rqrr(luma8_otsu));
        if decoded.len() > 0 {
            return Ok(decoded);
        }
        let luma8 = image.to_luma8();
        decoded.extend(with_rxing(&luma8));
        Ok(decoded)
    })
}

/// Scan QR codes from a grayscale image given in bytes.
#[pyfunction]
fn detect_and_decode_from_bytes(
    py: Python,
    data: Vec<u8>,
    width: u32,
    height: u32,
) -> PyResult<Vec<String>> {
    py.allow_threads(move || {
        let mut decoded: Vec<String> = Vec::new();
        if data.len() != (width * height) as usize {
            return PyResult::Err(PyValueError::new_err(
                "Data length does not match width and height",
            ));
        }
        let image_result = GrayImage::from_raw(width, height, data);
        let luma8 = match image_result {
            Some(image) => image,
            None => return PyResult::Err(PyValueError::new_err("Could not create image")),
        };
        let image = DynamicImage::from(luma8);
        let luma8_otsu = apply_threshold(&image);
        decoded.extend(with_rqrr(luma8_otsu));
        if decoded.len() > 0 {
            return Ok(decoded);
        }
        let luma8 = image.to_luma8();
        decoded.extend(with_rxing(&luma8));
        Ok(decoded)
    })
}

fn with_rqrr(luma8: ImageBuffer<Luma<u8>, Vec<u8>>) -> Vec<String> {
    let mut result = Vec::new();
    let mut prepared_image = rqrr::PreparedImage::prepare(luma8);
    let grids = prepared_image.detect_grids();
    for grid in grids.iter() {
        let decode_result = grid.decode();
        let (_meta, content) = match decode_result {
            Ok((meta, content)) => (meta, content),
            Err(_) => continue,
        };
        result.push(content.to_string());
    }
    result
}

fn with_rxing(luma8: &ImageBuffer<Luma<u8>, Vec<u8>>) -> Vec<String> {
    let mut result = Vec::new();

    let mut dch = DecodeHints {
        TryHarder: Some(true),
        ..Default::default()
    };

    let decode_result = rxing::helpers::detect_in_luma_with_hints(
        luma8.to_vec(),
        luma8.width(),
        luma8.height(),
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

/// qrlyzer QR code reader module.
#[pymodule]
fn qrlyzer(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(detect_and_decode, m)?)?;
    m.add_function(wrap_pyfunction!(detect_and_decode_from_bytes, m)?)?;
    Ok(())
}
