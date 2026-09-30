# qrlyzer 
[![CI](https://github.com/netlifeas/qrlyzer/actions/workflows/CI.yml/badge.svg)](https://github.com/netlifeas/qrlyzer/actions/workflows/CI.yml) [![Tests](https://github.com/netlifeas/qrlyzer/actions/workflows/tests.yml/badge.svg)](https://github.com/netlifeas/qrlyzer/actions/workflows/tests.yml) [![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT) ![PyPI - Python Version](https://img.shields.io/pypi/pyversions/qrlyzer?link=https%3A%2F%2Fpypi.org%2Fproject%2Fqrlyzer%2F) [![Sigstore Signed](https://img.shields.io/badge/signed%20with-sigstore-4b8bbe?logo=sigstore&logoColor=white)](https://github.com/netlifeas/qrlyzer/releases)


qrlyzer is a lightweight Python module for reading QR codes from images. It offers an optional auto-resizing feature that leverages fast_image_resize to utilize SIMD instructions for enhanced performance.

There is at least one top level domain that started using the name qrlyzer after this projects' conception and release. This project is not affiliated nor does it condone the use of the name.

## Getting Started

### Installing

Requires Python 3.11 or newer. CI tests Python 3.11–3.15, enabling prerelease
interpreter resolution only for Python 3.15. Source builds require maturin 1.15 or newer.

qrlyzer is available on PyPi. Install it with:

```bash
python -m pip install qrlyzer
```

### Basic usage:

#### Detecting QR codes from an image file

```python
import qrlyzer

# From path
qr_codes = qrlyzer.detect_and_decode("my_image.jpg")
print(f"Found QR codes: {qr_codes}")
```
#### Detecting QR codes from image bytes
```python
# From bytes
from PIL import Image
im = Image.open("my_image.jpg")
im = im.convert("L")
qr_codes = qrlyzer.detect_and_decode_from_bytes(im.tobytes(), im.width, im.height)
print(f"Found QR codes: {qr_codes}")
```

#### Using auto-resizing
Pass the ```auto_resize``` parameter to enable automatic resizing (from 100px to 1280px in the largest direction. 5 steps). These dimensions seem to be a good detection range for the libraries used.
```python
# Auto-scaling
qrlyzer.detect_and_decode("my_image.jpg", auto_resize=True)
```
Note: This can in some cases increase accuracy as well as speed, especially for large images where there is a QR code. If an image does not contain a QR code or the QR code is unreadable it will be slower.

Auto-resizing still decodes the full-resolution source image by default. For
optional reduced-resolution JPEG decoding, see [Performance suggestions](#performance-suggestions).

#### Getting decoded text with bounding boxes (`xywh`)
Use the bbox variants to get both content and coordinates. The bbox format is `(x, y, width, height)`.
Coordinates always refer to the original image dimensions, including when JPEG
IDCT scaling is used. Box edges can differ slightly between decoding scales.
```python
results = qrlyzer.detect_and_decode_with_bbox("my_image.jpg")
for content, (x, y, width, height) in results:
    print(content, x, y, width, height)
```

```python
results = qrlyzer.detect_and_decode_from_bytes_with_bbox(
    im.tobytes(), im.width, im.height
)
```

## Performance suggestions

Benchmark with representative images, including tiny QR codes and images without
codes. An optimization that helps successful scans can make unsuccessful ones slower.
If you already have grayscale pixels, pass them to the raw-byte APIs to avoid
decoding the same file again.

### Opt into reduced-resolution JPEG decoding

Both file APIs accept the keyword-only `jpeg_fast_path` option, **disabled by
default**. Enable it together with `auto_resize=True`:

```python
import qrlyzer

qr_codes = qrlyzer.detect_and_decode(
    "my_image.jpg", auto_resize=True, jpeg_fast_path=True
)
# Also supported by detect_and_decode_with_bbox; boxes use original coordinates.
```

The fast path uses `jpeg-decoder` to decode at 1/2, 1/4, or 1/8 dimensions when
the reduced image can retain at least 1280 pixels in one dimension. Large JPEGs
with sufficiently large QR codes can need substantially less time and memory.
Progressive JPEGs can benefit too, but generally save less memory.

If the scaled scan finds no codes, or the fast-path decoder cannot handle the
JPEG, qrlyzer retries with its existing full-resolution decoder and detection
pipeline. A tiny code or an image without codes therefore pays for an extra decode
and detection attempt. Leave the option off when these cases dominate your workload.
CMYK and smaller JPEGs keep the existing decoder. Non-JPEG files are unaffected,
and the option has no effect unless `auto_resize=True`. Raw-byte APIs do not accept it.

Detection stops at the first successful scan: a scaled result can omit smaller
codes elsewhere in a multi-code image. Fallback on an empty result does not make
this an exhaustive scan. Bounding boxes remain in original-image coordinates,
but their edges can differ slightly between decoding scales.

### Use Pillow `.draft()` before loading JPEG pixels

For applications already using Pillow, `Image.draft()` can request reduced-resolution
JPEG decoding and grayscale output. Call it **immediately after `Image.open()`**,
before `.load()`, `.convert()`, `.resize()`, or `.tobytes()` triggers decoding:

```python
from PIL import Image
import qrlyzer

path = "my_image.jpg"
with Image.open(path) as image:
    image.draft("L", (1280, 1280))
    gray = image.convert("L")
    qr_codes = qrlyzer.detect_and_decode_from_bytes(
        gray.tobytes(), gray.width, gray.height, auto_resize=True
    )

# Raw-byte APIs cannot recover pixels discarded by Pillow. Reopen the original
# through the file API if you want a full-resolution fallback on an empty result.
if not qr_codes:
    qr_codes = qrlyzer.detect_and_decode(path, auto_resize=True)
print(qr_codes)
```

The draft size is a hint, not an exact resize; use the resulting image's actual
dimensions. For example, a 6000×4000 JPEG may become 3000×2000 for this request,
not 1280×1280. Unsupported formats can ignore the request. Calling `.resize()`
after full decoding does not avoid the original decode time or peak pixel allocation.

With `detect_and_decode_from_bytes_with_bbox`, boxes refer to the supplied draft
pixels, not the original JPEG. To map them back, retain the original dimensions
before `.draft()` and scale box edges by each axis's original-to-draft ratio,
rounding left/top down and right/bottom up. The same tiny-code and multi-code
limitations apply as with the built-in fast path.


## Uses 

* [maturin](https://www.maturin.rs/) - Build & PyO3 bindings
* [rqrr](https://github.com/WanzenBug/rqrr/) - Reading QR codes
* [rxing](https://github.com/rxing-core/rxing/) - Reading QR codes
* [fast_image_resize](https://github.com/cykooz/fast_image_resize/) - Image resizing 
* [jpeg-decoder](https://github.com/image-rs/jpeg-decoder) - Reduced-resolution JPEG decoding

## Authors

* **Nikolai Ugelvik** - *Initial work* - [NikolaiUgelvik](https://github.com/NikolaiUgelvik)

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details

## Acknowledgments

* Thanks to all the contributors to the maturin, rqrr, rxing & fast_image_resize projects.
