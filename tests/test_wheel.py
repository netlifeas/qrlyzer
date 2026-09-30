from PIL import Image
import pytest
import qrlyzer


def _assert_bbox_within_image(
    bbox: tuple[int, int, int, int], width: int, height: int
) -> None:
    x, y, w, h = bbox
    assert x >= 0
    assert y >= 0
    assert w > 0
    assert h > 0
    assert x + w <= width
    assert y + h <= height


def test_detect_and_decode_success():
    output = qrlyzer.detect_and_decode("tests/fixtures/test.png")
    assert output == ["qrlyzer"]


def test_detect_and_decode_invalid_path():
    with pytest.raises(OSError):
        qrlyzer.detect_and_decode("tests/fixtures/invalid.png")


def test_detect_and_decode_needs_resize_success():
    output = qrlyzer.detect_and_decode(
        "tests/fixtures/test_resize.png", auto_resize=True
    )
    assert output == ["qrlyzer"]


def test_detect_and_decode_needs_resize_failure():
    output = qrlyzer.detect_and_decode("tests/fixtures/test_resize.png")
    assert output == []


def test_detect_and_decode_from_bytes_success():
    im = Image.open("tests/fixtures/test.png").convert("L")
    output = qrlyzer.detect_and_decode_from_bytes(im.tobytes(), im.width, im.height)
    assert output == ["qrlyzer"]


def test_detect_and_decode_from_bytes_failure():
    """Tests the case where the image is in the wrong mode.
    Image should be L, but is RGB."""
    im = Image.open("tests/fixtures/test.png")
    with pytest.raises(ValueError):
        qrlyzer.detect_and_decode_from_bytes(im.tobytes(), im.width, im.height)


def test_detect_and_decode_from_bytes_needs_resize_success():
    im = Image.open("tests/fixtures/test_resize.png").convert("L")
    output = qrlyzer.detect_and_decode_from_bytes(
        im.tobytes(), im.width, im.height, auto_resize=True
    )
    assert output == ["qrlyzer"]


def test_detect_and_decode_with_bbox_success():
    im = Image.open("tests/fixtures/test.png").convert("L")
    output = qrlyzer.detect_and_decode_with_bbox("tests/fixtures/test.png")
    assert len(output) == 1
    content, bbox = output[0]
    assert content == "qrlyzer"
    print(bbox)
    _assert_bbox_within_image(bbox, im.width, im.height)


def test_detect_and_decode_with_bbox_needs_resize_success():
    im = Image.open("tests/fixtures/test_resize.png").convert("L")
    output = qrlyzer.detect_and_decode_with_bbox(
        "tests/fixtures/test_resize.png", auto_resize=True
    )
    assert len(output) == 1
    content, bbox = output[0]
    assert content == "qrlyzer"
    _assert_bbox_within_image(bbox, im.width, im.height)


def test_detect_and_decode_from_bytes_with_bbox_success():
    im = Image.open("tests/fixtures/test.png").convert("L")
    output = qrlyzer.detect_and_decode_from_bytes_with_bbox(
        im.tobytes(), im.width, im.height
    )
    assert len(output) == 1
    content, bbox = output[0]
    assert content == "qrlyzer"
    _assert_bbox_within_image(bbox, im.width, im.height)


def test_detect_and_decode_from_bytes_with_bbox_needs_resize_success():
    im = Image.open("tests/fixtures/test_resize.png").convert("L")
    output = qrlyzer.detect_and_decode_from_bytes_with_bbox(
        im.tobytes(), im.width, im.height, auto_resize=True
    )
    assert len(output) == 1
    content, bbox = output[0]
    assert content == "qrlyzer"
    _assert_bbox_within_image(bbox, im.width, im.height)


@pytest.mark.parametrize(
    "mode,progressive,size,qr_size",
    [
        ("RGB", False, (6001, 4003), 1600),
        ("RGB", True, (6001, 4003), 1600),
        ("L", False, (6001, 4003), 1600),
        ("CMYK", False, (6001, 4003), 1600),
        ("RGB", False, (1024, 768), 400),
        ("RGB", False, (2000, 1333), 600),
    ],
)
def test_jpeg_detection_uses_original_coordinates(
    tmp_path, mode, progressive, size, qr_size
):
    image = Image.new("RGB", size, "white")
    code = Image.open("tests/fixtures/test.png").convert("RGB")
    code = code.resize((qr_size, qr_size), Image.Resampling.NEAREST)
    left, top = size[0] - qr_size - 100, size[1] - qr_size - 100
    image.paste(code, (left, top))
    path = tmp_path / "qr.jpg"
    image.convert(mode).save(path, quality=90, progressive=progressive)

    assert qrlyzer.detect_and_decode(
        str(path), auto_resize=True, jpeg_fast_path=True
    ) == ["qrlyzer"]
    results = qrlyzer.detect_and_decode_with_bbox(
        str(path), auto_resize=True, jpeg_fast_path=True
    )
    assert len(results) == 1
    content, (x, y, width, height) = results[0]
    assert content == "qrlyzer"
    _assert_bbox_within_image((x, y, width, height), *size)
    assert left <= x < left + qr_size // 3
    assert top <= y < top + qr_size // 3
    assert left + 2 * qr_size // 3 < x + width <= left + qr_size
    assert top + 2 * qr_size // 3 < y + height <= top + qr_size


def test_jpeg_tiny_qr_survives_full_resolution_fallback(tmp_path):
    image = Image.new("RGB", (6000, 4000), "white")
    code = Image.open("tests/fixtures/test.png").convert("RGB")
    image.paste(code.resize((100, 100), Image.Resampling.NEAREST), (2950, 1950))
    path = tmp_path / "tiny.jpg"
    image.save(path, quality=90)

    # The QR is unreadable at the IDCT preview size but readable in the source.
    preview = Image.open(path)
    preview.draft("L", (1500, 1000))
    preview = preview.convert("L")
    assert qrlyzer.detect_and_decode_from_bytes(
        preview.tobytes(), preview.width, preview.height, auto_resize=True
    ) == []
    expected = qrlyzer.detect_and_decode_with_bbox(str(path), auto_resize=False)
    assert [content for content, _ in expected] == ["qrlyzer"]
    assert qrlyzer.detect_and_decode_with_bbox(
        str(path), auto_resize=True, jpeg_fast_path=True
    ) == expected
    assert qrlyzer.detect_and_decode(
        str(path), auto_resize=True, jpeg_fast_path=True
    ) == ["qrlyzer"]


@pytest.mark.parametrize("with_bbox", [False, True])
def test_jpeg_no_qr_and_invalid_data(tmp_path, with_bbox):
    detect = (
        qrlyzer.detect_and_decode_with_bbox if with_bbox else qrlyzer.detect_and_decode
    )
    path = tmp_path / "blank.jpg"
    Image.new("RGB", (3000, 2000), "white").save(path)
    assert detect(str(path), auto_resize=True, jpeg_fast_path=True) == []
    path.write_bytes(b"\xff\xd8invalid JPEG")
    with pytest.raises(OSError):
        detect(str(path), auto_resize=True, jpeg_fast_path=True)
    with pytest.raises(OSError):
        detect(str(tmp_path / "missing.jpg"), auto_resize=True, jpeg_fast_path=True)


@pytest.mark.parametrize("auto_resize", [False, True])
def test_jpeg_default_matches_full_resolution_pixels(tmp_path, auto_resize):
    image = Image.new("L", (6001, 4003), 255)
    code = Image.open("tests/fixtures/test.png").convert("L")
    image.paste(code.resize((1600, 1600), Image.Resampling.NEAREST), (4301, 2303))
    path = tmp_path / "full.jpg"
    image.save(path, quality=90)
    with Image.open(path) as decoded:
        expected = qrlyzer.detect_and_decode_from_bytes_with_bbox(
            decoded.tobytes(), decoded.width, decoded.height, auto_resize=auto_resize
        )
    assert [text for text, _ in expected] == ["qrlyzer"]
    assert qrlyzer.detect_and_decode_with_bbox(
        str(path), auto_resize=auto_resize
    ) == expected
    assert qrlyzer.detect_and_decode_with_bbox(
        str(path), auto_resize=auto_resize, jpeg_fast_path=False
    ) == expected
    assert qrlyzer.detect_and_decode(str(path), auto_resize=auto_resize) == ["qrlyzer"]
    if not auto_resize:
        assert qrlyzer.detect_and_decode_with_bbox(
            str(path), jpeg_fast_path=True
        ) == expected
