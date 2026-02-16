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
