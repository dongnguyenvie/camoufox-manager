"""Draw the app icon (1024px) and pack it into AppIcon.icns. Run: python3 packaging/make_icon.py"""
import os, subprocess, sys, tempfile
from PIL import Image, ImageDraw, ImageFilter

S = 1024
here = os.path.dirname(os.path.abspath(__file__))

img = Image.new("RGBA", (S, S), (0, 0, 0, 0))
# macOS-style rounded background, gradient purple to blue
grad = Image.new("RGBA", (S, S))
gd = ImageDraw.Draw(grad)
for y in range(S):
    t = y / S
    gd.line([(0, y), (S, y)], fill=(int(99 + (37 - 99) * t), int(102 + (99 - 102) * t), int(241 + (235 - 241) * t), 255))
mask = Image.new("L", (S, S), 0)
ImageDraw.Draw(mask).rounded_rectangle([100, 100, S - 100, S - 100], radius=185, fill=255)
shadow = Image.new("RGBA", (S, S), (0, 0, 0, 0))
ImageDraw.Draw(shadow).rounded_rectangle([100, 118, S - 100, S - 82], radius=185, fill=(0, 0, 0, 90))
img.alpha_composite(shadow.filter(ImageFilter.GaussianBlur(18)))
img.paste(grad, (0, 0), mask)

d = ImageDraw.Draw(img)
# 3 "profile" rows stacked (like the table in the app), each row with a status dot
for i, y in enumerate((330, 500, 670)):
    d.rounded_rectangle([250, y - 58, 774, y + 58], radius=40, fill=(255, 255, 255, 235 if i == 0 else 150))
    d.ellipse([288, y - 26, 340, y + 26], fill=(34, 197, 94, 255) if i == 0 else (99, 102, 241, 200))
    d.rounded_rectangle([372, y - 14, 690 - i * 70, y + 14], radius=14, fill=(79, 70, 229, 200) if i == 0 else (99, 102, 241, 120))

with tempfile.TemporaryDirectory() as tmp:
    iconset = os.path.join(tmp, "AppIcon.iconset")
    os.makedirs(iconset)
    for size in (16, 32, 128, 256, 512):
        for scale in (1, 2):
            px = size * scale
            name = f"icon_{size}x{size}{'@2x' if scale == 2 else ''}.png"
            img.resize((px, px), Image.LANCZOS).save(os.path.join(iconset, name))
    out = os.path.join(here, "AppIcon.icns")
    subprocess.run(["iconutil", "-c", "icns", iconset, "-o", out], check=True)
    img.save(os.path.join(here, "icon.png"))
print("ok", out)
