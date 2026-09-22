"""Rasterize the synthetic cell buffers printed by the Rust renderer test."""
from pathlib import Path
import json, re
from PIL import Image, ImageDraw, ImageFont

root = Path(__file__).parent
items = [json.loads(line.removeprefix('OUTLINE_PREVIEW '))
         for line in (root/'preview.log').read_text().splitlines()
         if line.startswith('OUTLINE_PREVIEW ')]
assert len(items) == 2
(root/'selection-preview.json').write_text(json.dumps(items, indent=2))
font = ImageFont.truetype('/System/Library/Fonts/Menlo.ttc', 22)
heading = ImageFont.truetype('/System/Library/Fonts/Menlo.ttc', 20)
cw, ch = 15, 32
canvas = Image.new('RGB', (640, 530), '#fafafa')
draw = ImageDraw.Draw(canvas)

def color(value, default):
    match = re.fullmatch(r'Rgb\((\d+), (\d+), (\d+)\)', value)
    return tuple(map(int, match.groups())) if match else default

for panel, item in enumerate(items):
    oy = 55 + panel * 250
    draw.text((30, oy-30), ['Existing fill', 'Candidate outline — not installed'][panel], font=heading, fill='#303646')
    for index, cell in enumerate(item['cells']):
        col, row = index % item['width'], index // item['width']
        x, y = 30 + col*cw, oy + row*ch
        bg = color(cell['bg'], (250,250,250))
        fg = color(cell['fg'], (40,40,40))
        if 'DIM' in cell['modifier']:
            fg = tuple((f+b)//2 for f,b in zip(fg,bg))
        draw.rectangle((x,y,x+cw-1,y+ch-1), fill=bg)
        draw.text((x,y+3),cell['text'],font=font,fill=fg,stroke_width=0)
canvas.save(root/'selection-preview.png')
print(root/'selection-preview.png')
for line in (root/'preview.log').read_text().splitlines():
    if line.startswith('OUTLINE_SCALE '): print(line)
