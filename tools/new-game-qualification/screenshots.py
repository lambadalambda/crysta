"""Optional local PNGs (requires Pillow); no assets leave ignored local/."""
from pathlib import Path
import sys
from PIL import Image

for directory in sys.argv[1:]:
    directory = Path(directory)
    if not directory.resolve().is_relative_to(Path('local').resolve()):
        raise ValueError('captures must remain under local/')
    for path in sorted(directory.glob('f*.pixels')):
        raw = path.read_bytes()
        if len(raw) != 512 * 480 * 4:
            raise ValueError(f'{path}: expected 512x480 XRGB')
        # Little-endian XRGB bytes are B,G,R,X. Explicitly select even columns;
        # do not use nearest-neighbor downsampling (it can select odd columns).
        rgb = b''.join(raw[(y * 512 + 2 * x) * 4:(y * 512 + 2 * x) * 4 + 3][::-1]
                       for y in range(240) for x in range(256))
        Image.frombytes('RGB', (256, 240), rgb).resize(
            (768, 720), Image.Resampling.NEAREST).save(path.with_suffix('.png'))
