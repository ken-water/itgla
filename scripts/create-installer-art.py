"""Generate installer artwork using ITGLA's existing brand assets; no network needed."""
import argparse
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parents[1]
SCALE = 4
GREEN = '#203c30'
PAPER = '#f4f6f1'
RESAMPLE_LANCZOS = getattr(Image, 'Resampling', Image).LANCZOS


def generate(output):
    output.mkdir(parents=True, exist_ok=True)
    candidates = [Path('C:/Windows/Fonts/segoeui.ttf'),
                  Path('/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf')]
    font_path = next((p for p in candidates if p.exists()), None)
    if font_path is None:
        raise RuntimeError('Install Segoe UI or DejaVu Sans to generate installer artwork')
    def text(draw, xy, value, size, color):
        draw.text(tuple(int(v * SCALE) for v in xy), value,
                  font=ImageFont.truetype(str(font_path), size * SCALE), fill=color)
    def logo(im, xy, size):
        mark = Image.open(ROOT / 'assets/itgla.png').convert('RGBA')
        mark = mark.resize((size * SCALE, size * SCALE), RESAMPLE_LANCZOS)
        im.paste(mark, tuple(int(v * SCALE) for v in xy), mark)
    sidebar = Image.new('RGB', (164 * SCALE, 314 * SCALE), GREEN)
    logo(sidebar, (20, 26), 52)
    d = ImageDraw.Draw(sidebar)
    text(d, (20, 98), 'ITGLA', 25, PAPER)
    d.line([(20*SCALE, 145*SCALE), (144*SCALE, 145*SCALE)], fill='#8ba58c', width=SCALE)
    text(d, (20, 164), 'IT in a', 17, PAPER)
    text(d, (20, 191), 'glance.', 17, PAPER)
    header = Image.new('RGB', (150*SCALE, 57*SCALE), PAPER)
    logo(header, (12, 13), 30)
    text(ImageDraw.Draw(header), (52, 17), 'ITGLA', 17, GREEN)
    sidebar.save(output / 'sidebar.bmp')
    header.save(output / 'header.bmp')


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--output', type=Path, required=True)
    generate(parser.parse_args().output)
