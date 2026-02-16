use image::{GrayImage, Luma};
use pyo3::Python;
use qrlyzer::{
    detect_and_decode, detect_and_decode_from_bytes, detect_and_decode_from_bytes_with_bbox,
    detect_and_decode_with_bbox,
};

fn assert_bbox_within_image(bbox: (u32, u32, u32, u32), width: u32, height: u32) {
    let (x, y, w, h) = bbox;
    assert!(w > 0);
    assert!(h > 0);
    assert!(x + w <= width);
    assert!(y + h <= height);
}

#[test]
fn test_detect_and_decode_invalid_file() {
    Python::initialize();
    Python::attach(|py| {
        let result = detect_and_decode(py, "non_existent_file.png", false);
        assert!(result.is_err());
    });
}

#[test]
fn test_detect_and_decode_from_bytes_invalid_dimensions() {
    Python::initialize();
    Python::attach(|py| {
        // 10x10 image requires 100 bytes but using only 50 bytes
        let data = vec![0u8; 50];
        let result = detect_and_decode_from_bytes(py, data, 10, 10, false);
        assert!(result.is_err());
    });
}

#[test]
fn test_detect_and_decode_from_bytes_blank_image() {
    Python::initialize();
    Python::attach(|py| {
        let width = 10;
        let height = 10;
        let data = vec![0u8; (width * height) as usize];
        let result = detect_and_decode_from_bytes(py, data, width, height, false).unwrap();
        // No QR code expected in a blank image.
        assert!(result.is_empty());
    });
}

#[test]
fn test_detect_and_decode_blank_image_file() {
    Python::initialize();
    // Create a temporary blank grayscale image.
    let width = 10;
    let height = 10;
    let image: GrayImage = GrayImage::from_pixel(width, height, Luma([0u8]));
    let temp_dir = std::env::temp_dir();
    let file_path = temp_dir.join("test_blank.png");
    image.save(&file_path).unwrap();

    Python::attach(|py| {
        let result = detect_and_decode(py, file_path.to_str().unwrap(), false).unwrap();
        // Should return an empty vector as no QR code is present.
        assert!(result.is_empty());
    });

    // Cleanup the temporary file.
    std::fs::remove_file(file_path).unwrap();
}

#[test]
fn test_detect_and_decode_success_file() {
    Python::initialize();
    Python::attach(|py| {
        let file_path = "tests/fixtures/test.png";
        let result = detect_and_decode(py, file_path, false).unwrap();
        assert_eq!(result, vec!["qrlyzer".to_string()]);
    });
}

#[test]
fn test_detect_and_decode_success_bytes() {
    Python::initialize();
    Python::attach(|py| {
        let img = image::open("tests/fixtures/test.png").unwrap().to_luma8();
        let (width, height) = (img.width(), img.height());
        let data = img.into_vec();
        let result = detect_and_decode_from_bytes(py, data, width, height, false).unwrap();
        assert_eq!(result, vec!["qrlyzer".to_string()]);
    });
}

#[test]
fn test_detect_and_decode_success_file_requires_resize() {
    Python::initialize();
    Python::attach(|py| {
        let file_path = "tests/fixtures/test_resize.png";
        let result = detect_and_decode(py, file_path, true).unwrap();
        assert_eq!(result, vec!["qrlyzer".to_string()]);
    });
}

#[test]
fn test_detect_and_decode_failure_file_requires_resize() {
    Python::initialize();
    Python::attach(|py| {
        let file_path = "tests/fixtures/test_resize.png";
        let result = detect_and_decode(py, file_path, false).unwrap();
        assert_eq!(result, [] as [&str; 0]);
    });
}

#[test]
fn test_detect_and_decode_with_bbox_success_file() {
    Python::initialize();
    Python::attach(|py| {
        let file_path = "tests/fixtures/test.png";
        let image = image::open(file_path).unwrap().to_luma8();
        let result = detect_and_decode_with_bbox(py, file_path, false).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "qrlyzer");
        assert_bbox_within_image(result[0].1, image.width(), image.height());
    });
}

#[test]
fn test_detect_and_decode_with_bbox_success_file_requires_resize() {
    Python::initialize();
    Python::attach(|py| {
        let file_path = "tests/fixtures/test_resize.png";
        let image = image::open(file_path).unwrap().to_luma8();
        let result = detect_and_decode_with_bbox(py, file_path, true).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "qrlyzer");
        assert_bbox_within_image(result[0].1, image.width(), image.height());
    });
}

#[test]
fn test_detect_and_decode_from_bytes_with_bbox_success() {
    Python::initialize();
    Python::attach(|py| {
        let img = image::open("tests/fixtures/test.png").unwrap().to_luma8();
        let (width, height) = (img.width(), img.height());
        let data = img.into_vec();
        let result =
            detect_and_decode_from_bytes_with_bbox(py, data, width, height, false).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "qrlyzer");
        assert_bbox_within_image(result[0].1, width, height);
    });
}

#[test]
fn test_detect_and_decode_from_bytes_with_bbox_success_requires_resize() {
    Python::initialize();
    Python::attach(|py| {
        let img = image::open("tests/fixtures/test_resize.png")
            .unwrap()
            .to_luma8();
        let (width, height) = (img.width(), img.height());
        let data = img.into_vec();
        let result = detect_and_decode_from_bytes_with_bbox(py, data, width, height, true).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "qrlyzer");
        assert_bbox_within_image(result[0].1, width, height);
    });
}
