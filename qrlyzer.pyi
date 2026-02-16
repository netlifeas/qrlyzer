from os import PathLike
from typing import List, Tuple, TypeAlias, Union

BoundingBox: TypeAlias = Tuple[int, int, int, int]
DecodedQrWithBoundingBox: TypeAlias = Tuple[str, BoundingBox]

def detect_and_decode(
    image_path: Union[str, PathLike[str]], auto_resize: bool = False
) -> List[str]:
    """
    Scans an image file for QR codes.

    Loads the image from the given path, converts it to grayscale, and applies
    thresholding. If ``auto_resize`` is enabled, the image is resized at various
    scales to improve detection using multiple libraries.

    Args:
        image_path: The path to the image file.
        auto_resize: Whether to perform auto-resizing for enhanced detection.

    Returns:
        A list containing the decoded QR code strings.
    """
    ...

def detect_and_decode_from_bytes(
    data: bytes, width: int, height: int, auto_resize: bool = False
) -> List[str]:
    """
    Scans raw grayscale image bytes for QR codes.

    Constructs a grayscale image from the bytes and applies thresholding. If ``auto_resize`` is
    enabled, the image is resized at various scales to improve detection using multiple libraries.

    Args:
        data: Raw byte data of the grayscale image.
        width: The width of the image.
        height: The height of the image.
        auto_resize: Whether to perform auto-resizing for enhanced detection.

    Returns:
        A list containing the decoded QR code strings.
    """
    ...

def detect_and_decode_with_bbox(
    image_path: Union[str, PathLike[str]], auto_resize: bool = False
) -> List[DecodedQrWithBoundingBox]:
    """
    Scans an image file for QR codes and returns decoded values with bounding boxes.

    Bounding boxes are returned in ``(x, y, width, height)`` format where
    ``x`` and ``y`` are the top-left pixel coordinates.

    Args:
        image_path: The path to the image file.
        auto_resize: Whether to perform auto-resizing for enhanced detection.

    Returns:
        A list of ``(decoded_text, (x, y, width, height))`` tuples.
    """
    ...

def detect_and_decode_from_bytes_with_bbox(
    data: bytes, width: int, height: int, auto_resize: bool = False
) -> List[DecodedQrWithBoundingBox]:
    """
    Scans raw grayscale image bytes for QR codes and returns decoded values with bounding boxes.

    Bounding boxes are returned in ``(x, y, width, height)`` format where
    ``x`` and ``y`` are the top-left pixel coordinates.

    Args:
        data: Raw byte data of the grayscale image.
        width: The width of the image.
        height: The height of the image.
        auto_resize: Whether to perform auto-resizing for enhanced detection.

    Returns:
        A list of ``(decoded_text, (x, y, width, height))`` tuples.
    """
    ...
