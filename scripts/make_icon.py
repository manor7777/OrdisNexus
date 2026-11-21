#!/usr/bin/env python3
"""Render assets/icon.png and assets/icon.ico (Fluent-style swap/compare glyph)."""
from pathlib import Path
from PIL import Image, ImageDraw, ImageFilter

S = 1024
out = Path(__file__).resolve().parent.parent / "assets"

# background: rounded square, diagonal blue gradient
bg = Image.new("RGBA", (S, S))
px = bg.load()
c1, c2 = (0, 90, 190), (96, 205, 255)
for y in range(S):
    for x in range(S):
        t = (x + y) / (2 * S)
        px[x, y] = tuple(int(c1[i] + (c2[i] - c1[i]) * t) for i in range(3)) + (255,)
mask = Image.new("L", (S, S), 0)
ImageDraw.Draw(mask).rounded_rectangle((40, 40, S - 40, S - 40), radius=230, fill=255)
img = Image.new("RGBA", (S, S), (0, 0, 0, 0))
img.paste(bg, (0, 0), mask)

# soft shadow + white swap arrows
glyph = Image.new("RGBA", (S, S), (0, 0, 0, 0))
d = ImageDraw.Draw(glyph)
W = 70  # stroke
def arrow(y, left_to_right):
    x0, x1 = (250, 770) if left_to_right else (774, 254)
    d.line((x0, y, x1, y), fill="white", width=W)
    d.ellipse((x0 - W / 2, y - W / 2, x0 + W / 2, y + W / 2), fill="white")
    s = 1 if left_to_right else -1
    tip = x1 + s * 70
    d.polygon([(tip, y), (x1 - s * 60, y - 150), (x1 - s * 60, y + 150)], fill="white")
arrow(400, True)
arrow(624, False)
shadow = glyph.copy().filter(ImageFilter.GaussianBlur(18))
sh = Image.new("RGBA", (S, S), (0, 0, 0, 0))
sh.paste((0, 30, 80, 120), (0, 14), shadow.split()[3])
img = Image.alpha_composite(img, sh)
img = Image.alpha_composite(img, glyph)

img.resize((256, 256), Image.LANCZOS).save(out / "icon.png")
img.save(out / "icon.ico", sizes=[(256, 256), (128, 128), (64, 64), (48, 48), (32, 32), (16, 16)])
print("ok")
