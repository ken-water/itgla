"""Generate installer artwork using ITGLA's existing brand assets; no network needed."""
import argparse
from pathlib import Path
from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
SCALE = 4
GREEN = '#203c30'
RESAMPLE_LANCZOS = getattr(Image, 'Resampling', Image).LANCZOS


def generate(output):
    output.mkdir(parents=True, exist_ok=True)
    def logo(im, xy, size):
        mark = Image.open(ROOT / 'assets/itgla.png').convert('RGBA')
        mark = mark.resize((size * SCALE, size * SCALE), RESAMPLE_LANCZOS)
        im.paste(mark, tuple(int(v * SCALE) for v in xy), mark)
    # Keep text in native NSIS controls; only the logo belongs in scaled artwork.
    sidebar = Image.new('RGB', (164 * SCALE, 314 * SCALE), GREEN)
    logo(sidebar, ((164 - 64) // 2, (314 - 64) // 2), 64)
    sidebar.save(output / 'sidebar.bmp')


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--output', type=Path, required=True)
    generate(parser.parse_args().output)
