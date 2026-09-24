"""Generate the Zenou Tweaks icon set (Tauri defaults).

Small sizes (16-48px, what Windows shortcuts/taskbar actually display) are drawn
directly on a pixel grid — crisp strokes, no downscale blur. Large sizes use a
supersampled render for smooth curves.
"""
from PIL import Image, ImageDraw
import os

OUT = os.path.join(os.path.dirname(__file__), "..", "src-tauri", "icons")
os.makedirs(OUT, exist_ok=True)

BG = (11, 13, 15, 255)        # #0b0d0f
TEAL = (45, 212, 191, 255)    # #2dd4bf
BORDER = (45, 212, 191, 90)


def render_aa(size: int) -> Image.Image:
    """Supersampled anti-aliased render for large sizes."""
    s = 8
    S = size * s
    img = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    rx = int(S * 28 / 128)
    inset = int(S * 4 / 128)
    d.rounded_rectangle((inset, inset, S - inset, S - inset), radius=rx, fill=BG)
    bw = max(1, int(S * 2 / 128))
    d.rounded_rectangle((inset, inset, S - inset, S - inset), radius=rx, outline=BORDER, width=bw)
    # Z mark
    w = int(S * 11 / 128)
    x0, x1 = int(S * 40 / 128), int(S * 88 / 128)
    y0, y1 = int(S * 34 / 128), int(S * 94 / 128)
    d.line([(x0, y0), (x1, y0)], fill=TEAL, width=w)
    d.line([(x1, y0), (x0, y1)], fill=TEAL, width=w)
    d.line([(x0, y1), (x1, y1)], fill=TEAL, width=w)
    r = w // 2
    for (cx, cy) in [(x0, y0), (x1, y0), (x1, y1), (x0, y1)]:
        d.ellipse((cx - r, cy - r, cx + r, cy + r), fill=TEAL)
    return img.resize((size, size), Image.LANCZOS)


def render_crisp(size: int) -> Image.Image:
    """Direct pixel-grid render for small sizes: no resampling, hard edges.

    Stroke widths are chosen so the Z stays legible; coordinates snap to the
    pixel grid so horizontal/diagonal strokes land on whole pixels.
    """
    img = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)

    # Background rounded rect with a small inset, radius scaled.
    inset = max(1, round(size * 4 / 128))
    rx = max(2, round(size * 28 / 128))
    d.rounded_rectangle((inset, inset, size - 1 - inset, size - 1 - inset), radius=rx, fill=BG)

    # Z stroke width: 16px -> 2, 24 -> 3, 32 -> 3, 48 -> 4, else ~ size/12.
    w = max(2, round(size / 12))
    # Z geometry snapped to pixels, generous margins so it never touches the edge.
    m = max(3, round(size * 40 / 128))          # left margin
    right = size - 1 - m                        # right x
    top = max(3, round(size * 34 / 128))        # top y
    bot = size - 1 - max(3, round(size * 34 / 128))  # bottom y

    # Top horizontal bar
    d.rectangle((m, top, right, top + w - 1), fill=TEAL)
    # Diagonal from right/top to left/bottom — stepped line, width-aware
    steps = bot - top
    for i in range(steps + 1):
        y = top + i
        # Ease the diagonal: interpolate x from right to left
        x = round(right - (right - m) * (i / steps))
        d.rectangle((x, y, min(x + w - 1, right), y), fill=TEAL)
    # Bottom horizontal bar
    d.rectangle((m, bot - w + 1, right, bot), fill=TEAL)
    # Round caps for legibility at tiny sizes
    r = w // 2
    for (cx, cy) in [(m, top), (right, top), (m, bot), (right, bot)]:
        d.ellipse((cx - r, cy - r, cx + r, cy + r), fill=TEAL)

    return img


def render(size: int) -> Image.Image:
    if size <= 48:
        return render_crisp(size)
    return render_aa(size)


sizes = {
    "32x32.png": 32,
    "128x128.png": 128,
    "128x128@2x.png": 256,
    "icon.png": 512,
}
for name, size in sizes.items():
    render(size).save(os.path.join(OUT, name))

# icon.ico with multiple sizes — each frame rendered natively, never downscaled.
# Pillow's ICO writer only accepts sizes <= the base image's size, so the largest
# frame must be the base; exact-size frames are then used for each entry.
frames = [render(sz) for sz in (256, 128, 64, 48, 32, 24, 16)]
frames[0].save(
    os.path.join(OUT, "icon.ico"),
    format="ICO",
    sizes=[(f.width, f.height) for f in frames],
    append_images=frames[1:],
)

print("icons written to", os.path.abspath(OUT))
