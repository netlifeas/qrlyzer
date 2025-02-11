# qrlyzer 
[![CI](https://github.com/netlifeas/qrlyzer/actions/workflows/CI.yml/badge.svg)](https://github.com/netlifeas/qrlyzer/actions/workflows/CI.yml) [![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

qrlyzer is an extremely simple and lightweight Python module for reading QR codes from images.

## Getting Started

Example usage:

```python
import qrlyzer

# From path
qr_codes = qrlyzer.detect_and_decode("my_image.jpg")
print(f"Found QR codes: {qr_codes}")

# From bytes
from PIL import Image
im = Image.open("my_image.jpg")
im = im.convert("L")
qr_codes = qrlyzer.detect_and_decode_from_bytes(im.to_bytes(), im.width, im.height)
print(f"Found QR codes: {qr_codes}")
```

### Installing

qrlyzer is available on PyPi. Install it with:

```bash
python -m pip install qrlyzer
```

## Uses 

* [maturin](https://www.maturin.rs/) - Build & PyO3 bindings
* [rqrr](https://github.com/WanzenBug/rqrr/) - Reading QR codes
* [rxing](https://github.com/rxing-core/rxing/) - Reading QR codes

## Authors

* **Nikolai Ugelvik** - *Initial work* - [NikolaiUgelvik](https://github.com/NikolaiUgelvik)

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details

## Acknowledgments

* Thanks to all the contributors to the maturin, rqrr and rxing projects.

