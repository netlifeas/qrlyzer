from os import PathLike
from typing import List, Union

def detect_and_decode(
    image_path: Union[str, PathLike[str]], auto_resize: bool = False
) -> List[str]: ...
def detect_and_decode_from_bytes(
    data: bytes, width: int, height: int, auto_resize: bool = False
) -> List[str]: ...
