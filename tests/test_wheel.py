from PIL import Image
import pytest
import qrlyzer


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
