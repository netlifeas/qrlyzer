use fast_image_resize as fr;
use image::GrayImage;
use imageproc::contrast::{otsu_level, threshold, ThresholdType};
use pyo3::{
    exceptions::{PyIOError, PyValueError},
    prelude::*,
};
use rqrr;
use rxing::{self, BarcodeFormat, DecodeHints};

/// Scan QR codes from an image given as a path.
#[pyfunction]
#[pyo3(signature = (path, auto_resize=false))]
fn detect_and_decode(py: Python, path: &str, auto_resize: bool) -> PyResult<Vec<String>> {
    py.allow_threads(move || {
        let mut decoded: Vec<String> = Vec::new();
        let image = load_image(path)?;
        if let Some(result) = do_detect_and_decode(&image, auto_resize) {
            decoded.extend(result);
        }
        Ok(decoded)
    })
}

/// Scan QR codes from a grayscale image given in bytes.
#[pyfunction]
#[pyo3(signature = (data, width, height, auto_resize=false))]
fn detect_and_decode_from_bytes(
    py: Python,
    data: Vec<u8>,
    width: u32,
    height: u32,
    auto_resize: bool,
) -> PyResult<Vec<String>> {
    py.allow_threads(move || {
        let mut decoded: Vec<String> = Vec::new();
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
        if let Some(result) = do_detect_and_decode(&image, auto_resize) {
            decoded.extend(result);
        }
        Ok(decoded)
    })
}

fn do_detect_and_decode(image: &GrayImage, auto_resize: bool) -> Option<Vec<String>> {
    let mut decoded: Vec<String> = Vec::new();
    if auto_resize {
        // This range seems to be a good detection range based on tests
        let min_scale = 100.0 / (image.width().max(image.height())) as f32;
        let max_scale = 1280.0 / (image.width().max(image.height())) as f32;
        let scale_steps = 5;

        let scale_src_result = fr::images::Image::from_vec_u8(
            image.width(),
            image.height(),
            image.to_vec(),
            fr::PixelType::U8,
        );
        let scale_src = match scale_src_result {
            Ok(image) => image,
            Err(_) => return None,
        };

        for scale in (0..=scale_steps)
            .rev()
            .map(|step| min_scale + (max_scale - min_scale) * step as f32 / scale_steps as f32)
        {
            let resized = resize_image(&scale_src, scale);
            if let Some(resized) = resized {
                let luma8_otsu = apply_threshold(&resized);
                decoded.extend(with_rqrr(luma8_otsu));
                if decoded.len() > 0 {
                    return Some(decoded);
                }
                decoded.extend(with_rxing(&resized));
                if decoded.len() > 0 {
                    break;
                }
            }
        }
    }
    let luma8_otsu = apply_threshold(&image);
    decoded.extend(with_rqrr(luma8_otsu));
    if decoded.len() > 0 {
        return Some(decoded);
    }
    decoded.extend(with_rxing(&image));
    Some(decoded)
}

fn with_rqrr(image: GrayImage) -> Vec<String> {
    let mut result = Vec::new();
    let mut prepared_image = rqrr::PreparedImage::prepare(image);
    let grids = prepared_image.detect_grids();
    for grid in grids.into_iter() {
        let decode_result = grid.decode();
        let (_meta, content) = match decode_result {
            Ok((meta, content)) => (meta, content),
            Err(_) => continue,
        };
        result.push(content.to_string());
    }
    result
}

fn with_rxing(image: &GrayImage) -> Vec<String> {
    let mut result = Vec::new();

    let mut dch = DecodeHints {
        TryHarder: Some(true),
        ..Default::default()
    };

    let decode_result = rxing::helpers::detect_in_luma_with_hints(
        image.to_vec(),
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

fn load_image(path: &str) -> PyResult<GrayImage> {
    let image = image::open(path);
    match image {
        Ok(image) => Ok(image.to_luma8()),
        Err(_) => return PyResult::Err(PyIOError::new_err("Could not load image")),
    }
}

fn apply_threshold(image: &GrayImage) -> GrayImage {
    let ol = otsu_level(&image);
    threshold(&image, ol, ThresholdType::Binary)
}

fn resize_image(image: &fr::images::Image, target_scale: f32) -> Option<GrayImage> {
    let mut dst_image = fr::images::Image::new(
        (image.width() as f32 * target_scale) as u32,
        (image.height() as f32 * target_scale) as u32,
        fr::PixelType::U8,
    );
    let mut resizer = fr::Resizer::new();
    let dst_image = match resizer.resize(image, &mut dst_image, &fr::ResizeOptions::default()) {
        Ok(_) => dst_image,
        Err(_) => return None,
    };
    GrayImage::from_raw(
        dst_image.width(),
        dst_image.height(),
        dst_image.buffer().to_vec(),
    )
}

/// qrlyzer QR code reader module.
#[pymodule(gil_used = false)]
fn qrlyzer(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(detect_and_decode, m)?)?;
    m.add_function(wrap_pyfunction!(detect_and_decode_from_bytes, m)?)?;
    Ok(())
}
