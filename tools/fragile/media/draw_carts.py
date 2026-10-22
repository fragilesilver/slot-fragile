#!/usr/bin/env python3
"""Draws the neutral NES and SNES shells, assets/nes_neutral.png and assets/snes_neutral.png.

Fragile: drawn after photographs of the real carts (an NTSC SNES cart and an NES cart) rather
than unslot's renders, with no maker's marks. The plastic is grey around 128, which
slot-ui's cart.rs `overlay` turns into the game's colour; the SNES sticker keeps its own black
and red (`sticker` in media.rs). Pure Python, no dependencies:

    python3 tools/fragile/media/draw_carts.py
"""

import math
import os
import random
import struct
import sys
import zlib

SS = 4  # samples a side per pixel


class Canvas:
    def __init__(self, w, h):
        self.w, self.h = w, h
        self.rgb = [[0.0, 0.0, 0.0] for _ in range(w * h)]
        self.a = [0.0] * (w * h)

    def paint(self, inside, box, colour, outline=False, alpha=1.0):
        """Covers what `inside(x, y)` holds in box (x0, y0, x1, y1) with colour, a grey or an
        RGB triple, or a function of (x, y) giving one."""
        x0, y0, x1, y1 = box
        for py in range(max(0, int(math.floor(y0))), min(self.h, int(math.ceil(y1)))):
            for px in range(max(0, int(math.floor(x0))), min(self.w, int(math.ceil(x1)))):
                n = 0
                for sy in range(SS):
                    for sx in range(SS):
                        if inside(px + (sx + 0.5) / SS, py + (sy + 0.5) / SS):
                            n += 1
                if n == 0:
                    continue
                c = n / (SS * SS) * alpha
                i = py * self.w + px
                col = colour(px + 0.5, py + 0.5) if callable(colour) else colour
                if not isinstance(col, (tuple, list)):
                    col = (col, col, col)
                d = self.rgb[i]
                for k in range(3):
                    d[k] = d[k] * (1 - c) + col[k] * c
                if outline:
                    self.a[i] = max(self.a[i], c)

    def rrect(self, x0, y0, x1, y1, r, colour, outline=False, alpha=1.0, radii=None):
        tl, tr, br, bl = radii or (r, r, r, r)

        def inside(x, y):
            if not (x0 <= x < x1 and y0 <= y < y1):
                return False
            for rr, cx, cy, left, top in (
                (tl, x0 + tl, y0 + tl, True, True),
                (tr, x1 - tr, y0 + tr, False, True),
                (br, x1 - br, y1 - br, False, False),
                (bl, x0 + bl, y1 - bl, True, False),
            ):
                if rr <= 0:
                    continue
                if (x < cx if left else x > cx) and (y < cy if top else y > cy):
                    return (x - cx) ** 2 + (y - cy) ** 2 <= rr * rr
            return True

        self.paint(inside, (x0, y0, x1, y1), colour, outline, alpha)

    def circle(self, cx, cy, r, colour, alpha=1.0):
        self.paint(
            lambda x, y: (x - cx) ** 2 + (y - cy) ** 2 <= r * r,
            (cx - r, cy - r, cx + r, cy + r),
            colour,
            alpha=alpha,
        )

    def tri_outline(self, pts, width, colour):
        """The edges of a triangle, `width` thick."""
        def near(x, y):
            for (ax, ay), (bx, by) in zip(pts, pts[1:] + pts[:1]):
                dx, dy = bx - ax, by - ay
                t = max(0.0, min(1.0, ((x - ax) * dx + (y - ay) * dy) / (dx * dx + dy * dy)))
                if math.hypot(x - ax - t * dx, y - ay - t * dy) <= width / 2:
                    return True
            return False

        xs = [p[0] for p in pts]
        ys = [p[1] for p in pts]
        self.paint(near, (min(xs) - width, min(ys) - width, max(xs) + width, max(ys) + width), colour)

    def clear(self, x0, y0, x1, y1):
        """Cuts a notch out of the outline."""
        for py in range(y0, y1):
            for px in range(x0, x1):
                self.a[py * self.w + px] = 0.0

    def png(self, path):
        raw = bytearray()
        for y in range(self.h):
            raw.append(0)
            for x in range(self.w):
                i = y * self.w + x
                a = self.a[i]
                for k in range(3):
                    raw.append(max(0, min(255, round(self.rgb[i][k]))))
                raw.append(max(0, min(255, round(a * 255))))

        def chunk(kind, data):
            body = kind + data
            return struct.pack(">I", len(data)) + body + struct.pack(">I", zlib.crc32(body))

        with open(path, "wb") as f:
            f.write(b"\x89PNG\r\n\x1a\n")
            f.write(chunk(b"IHDR", struct.pack(">IIBBBBB", self.w, self.h, 8, 6, 0, 0, 0)))
            f.write(chunk(b"IDAT", zlib.compress(bytes(raw), 9)))
            f.write(chunk(b"IEND", b""))


def plastic(h, top=138, bottom=120, seed=1):
    """Grey plastic lit from above, with a fine grain."""
    rng = random.Random(seed)
    grain = {}

    def colour(x, y):
        key = (int(x), int(y))
        if key not in grain:
            grain[key] = rng.uniform(-2.5, 2.5)
        return top + (bottom - top) * (y / h) + grain[key]

    return colour


LIGHT, SHADE = 170, 78


def raised_h(c, x0, x1, y, light=LIGHT, shade=SHADE):
    """A horizontal ridge: lit above, shaded below."""
    c.rrect(x0, y - 1.2, x1, y, 0, light, alpha=0.75)
    c.rrect(x0, y, x1, y + 1.4, 0, shade, alpha=0.8)


def groove_h(c, x0, x1, y):
    """A horizontal groove: shaded above, lit below."""
    c.rrect(x0, y - 1.4, x1, y, 0, SHADE, alpha=0.85)
    c.rrect(x0, y, x1, y + 1.0, 0, LIGHT, alpha=0.6)


def groove_v(c, x, y0, y1):
    c.rrect(x - 1.2, y0, x, y1, 0, SHADE, alpha=0.85)
    c.rrect(x, y0, x + 1.0, y1, 0, LIGHT, alpha=0.55)


def snes():
    W, H = 360, 236
    c = Canvas(W, H)
    body = plastic(H, seed=2)
    wing_top = 9
    # The ribbed wings either side, six ribs each.
    pitch = (H - wing_top) / 6
    for side in (0, 1):
        x0, x1 = (0, 62) if side == 0 else (298, W)
        for k in range(6):
            y0 = wing_top + k * pitch + (1.5 if k else 0)
            y1 = wing_top + (k + 1) * pitch - 1.5
            outer = 5
            radii = (outer, 0, 0, outer) if side == 0 else (0, outer, outer, 0)
            if k == 5:
                radii = (outer, 0, 0, 7) if side == 0 else (0, outer, 7, 0)
            c.rrect(x0, y0, x1, y1, 0, body, outline=True, radii=radii)
            raised_h(c, x0 + 2, x1 - 2, y0 + 1.2, light=165, shade=128)
            c.rrect(x0 + 2, y1 - 2.2, x1 - 2, y1, 0, 92, alpha=0.6)
        # The gaps between ribs: deep.
        for k in range(1, 6):
            y = wing_top + k * pitch
            c.rrect(x0, y - 1.5, x1, y + 1.5, 0, 52, outline=True, alpha=1.0)
    # The raised centre.
    c.rrect(56, 0, 304, H, 0, body, outline=True, radii=(12, 12, 6, 6))
    c.rrect(58, 1, 302, 3, 0, LIGHT, alpha=0.55)
    for x in (56, 304):
        c.rrect(x - 2.5, wing_top, x, H, 0, SHADE, alpha=0.7) if x == 56 else c.rrect(
            x, wing_top, x + 2.5, H, 0, SHADE, alpha=0.7
        )
    c.rrect(56, H - 2, 304, H, 0, 96, alpha=0.6)
    # The sticker, in its recess.
    sx0, sy0, sx1, sy1 = 70, 4, 290, 108
    c.rrect(sx0 - 2, sy0 - 1, sx1 + 2, sy1 + 2.5, 5, 70, alpha=0.8)
    c.rrect(sx0 - 1, sy1, sx1 + 1, sy1 + 2, 2, LIGHT, alpha=0.5)
    c.rrect(sx0, sy0, sx1, sy1, 4, (22, 22, 24))
    # Its corner marks: a little lozenge top left and a pointer below it.
    c.rrect(76, 12, 100, 20, 4, (210, 40, 46))
    c.rrect(78, 13.5, 98, 18.5, 3, (236, 236, 236))
    c.paint(lambda x, y: 0 <= y - 30 <= 5 and abs(x - 82) <= (35 - y) * 0.9, (76, 29, 88, 36),
            (110, 90, 120))
    # The art window, left of centre (the label box in media.rs).
    c.rrect(91, 8, 245, 96, 1, (8, 8, 8))
    # The red band top right, the hatched emblem under it, the red name bar at the foot.
    c.rrect(252, 8, 286, 14, 0, (214, 30, 40))
    emblem = (258, 38, 284, 70)

    def hatch(x, y):
        return (int(x + y) // 3) % 2 == 0

    c.paint(lambda x, y: emblem[0] <= x < emblem[2] and emblem[1] <= y < emblem[3] and hatch(x, y),
            emblem, (62, 62, 66))
    c.rrect(252, 86, 286, 96, 1, (214, 30, 40))
    c.rrect(252, 98, 286, 100, 0, (214, 30, 40), alpha=0.7)
    # The slot across the lower front, and the two screws.
    c.rrect(70, 142, 290, 158, 8, 120 - 50)
    c.rrect(72, 142, 288, 147, 6, 40, alpha=0.7)
    c.rrect(76, 156, 284, 158, 2, LIGHT, alpha=0.45)
    # Under it, a shallow box down the middle, so the two make a T.
    c.rrect(148, 156, 212, 214, 4, 104, alpha=0.8, radii=(0, 0, 4, 4))
    c.rrect(148, 156, 150.5, 214, 0, SHADE, alpha=0.6)
    c.rrect(148, 211.5, 212, 214, 0, LIGHT, alpha=0.5, radii=(0, 0, 4, 4))
    c.rrect(209.5, 156, 212, 212, 0, LIGHT, alpha=0.4)
    for x in (31, 329):
        c.circle(x, 216, 6.5, 150)
        c.circle(x, 216, 5.2, 46)
        c.circle(x, 216, 2.6, 86)
    return c, (91, 8, 154, 88), (sx0, sy0, sx1 - sx0, sy1 - sy0)


def nes():
    W, H = 304, 337
    c = Canvas(W, H)
    body = plastic(H, top=124, bottom=110, seed=3)
    top = 7  # the top edge, with a tab standing above it near each corner
    c.rrect(0, top, W, H, 2, body, outline=True)
    for x0, x1 in ((3, 15), (W - 30, W - 17)):
        c.rrect(x0, 0, x1, top + 1, 1, body, outline=True, radii=(1, 1, 0, 0))
        c.rrect(x0 + 1, 0.5, x1 - 1, 1.8, 0, LIGHT, alpha=0.6)
        c.rrect(x1 - 1.5, 1, x1, top, 0, SHADE, alpha=0.6)
    c.rrect(1, top, W - 1, top + 1.5, 0, LIGHT, alpha=0.5)
    # The foot: the rails either side stop short of the middle, which runs on into the console.
    foot = H - 18
    left_rail, right_rail = 46, 266
    c.clear(0, foot, left_rail, H)
    c.clear(right_rail, foot, W, H)
    c.rrect(0, foot - 2, left_rail, foot, 0, 92, alpha=0.6)
    c.rrect(right_rail, foot - 2, W, foot, 0, 92, alpha=0.6)
    c.rrect(left_rail, H - 2, right_rail, H, 0, 92, alpha=0.6)
    c.rrect(left_rail, foot, left_rail + 1.5, H, 0, LIGHT, alpha=0.4)
    c.rrect(right_rail - 1.5, foot, right_rail, H, 0, SHADE, alpha=0.6)
    # The plain rail down the left, then the ribbed grip.
    gx0, gx1 = 49, 114
    groove_v(c, gx0 - 1, top + 1, H - 3)
    groove_v(c, gx1, top + 1, H - 3)
    # A pocket at the top of the grip, open at the top edge, with ribs down inside it.
    pocket = 40
    c.rrect(gx0 + 1, top, gx1 - 1, pocket, 0, 84)
    c.rrect(gx0 + 1, top, gx0 + 4, pocket, 0, 56, alpha=0.7)
    c.rrect(gx0 + 1, top, gx1 - 1, top + 3, 0, 60, alpha=0.6)
    for k in range(3):
        y = top + 9 + k * 10
        c.rrect(gx0 + 5, y, gx1 - 3, y + 1.2, 0, 112, alpha=0.8)
        c.rrect(gx0 + 5, y + 1.2, gx1 - 3, y + 2.4, 0, 60, alpha=0.7)
    c.rrect(gx0 + 1, pocket, gx1 - 1, pocket + 1.5, 0, LIGHT, alpha=0.7)
    gy0, gy1 = pocket + 6, 288
    ribs = 24
    pitch = (gy1 - gy0) / ribs
    for k in range(ribs + 1):
        raised_h(c, gx0 + 1, gx1 - 1, gy0 + k * pitch)
    # The plain panel at the foot of the grip.
    groove_h(c, gx0 + 1, gx1 - 1, gy1 + 6)
    # The label, in a recess that runs up to the top edge.
    lx0, ly0, lx1, ly1 = 122, top + 2, 260, 218
    c.rrect(lx0 - 3, top, lx1 + 3, ly1 + 3, 0, 98, alpha=0.9, radii=(0, 0, 3, 3))
    c.rrect(lx0 - 3, top, lx0 - 1, ly1 + 3, 0, 70, alpha=0.7)
    c.rrect(lx0 - 2, ly1 + 1, lx1 + 2, ly1 + 3, 2, LIGHT, alpha=0.6)
    c.rrect(lx1 + 1, top, lx1 + 3, ly1 + 3, 0, LIGHT, alpha=0.5)
    c.rrect(lx0, ly0, lx1, ly1, 1, 200)
    # The rail down the right, and the arrow pressed in about mid-width.
    groove_v(c, right_rail, top + 1, foot)
    mid = W * 0.495
    tri = [(mid - 12, 238.0), (mid + 12, 238.0), (mid, 257.0)]
    c.tri_outline([(x, y + 1.2) for x, y in tri], 2.0, LIGHT)
    c.tri_outline(tri, 2.0, SHADE)
    return c, (lx0, ly0, lx1 - lx0, ly1 - ly0), None


def main():
    root = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "..")
    out = sys.argv[1] if len(sys.argv) > 1 else os.path.join(root, "assets")
    for name, draw in (("snes", snes), ("nes", nes)):
        c, label, sticker = draw()
        c.png(os.path.join(out, f"{name}_neutral.png"))
        print(f"{name}: {c.w}x{c.h} label {label} sticker {sticker}")


if __name__ == "__main__":
    main()
