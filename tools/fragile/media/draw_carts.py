#!/usr/bin/env python3
"""Draws the neutral NES and SNES shells, assets/nes_neutral.png and assets/snes_neutral.png.

Fragile: modelled after photographs of the real carts (an NTSC SNES cart and an NES cart),
with no maker's marks. Each shell is a height field (its ribs, grooves, recesses and rounded
edges) lit from the top left like the Game Boy family's shells, with their speckled grain. Flat
plastic comes out at 128, which slot-ui's cart.rs `overlay` turns into the game's colour; the
SNES sticker keeps its own black and red (`sticker` in media.rs). Pure Python, no dependencies:

    python3 tools/fragile/media/draw_carts.py [OUT_DIR]
"""

import math
import os
import random
import struct
import sys
import zlib

SS = 4  # samples a side per pixel when covering a shape

# Light from the top left and a little in front, as on the Game Boy shells.
_L = (-0.38, -0.62, 0.69)
_n = math.sqrt(sum(v * v for v in _L))
LIGHT = tuple(v / _n for v in _L)
FLAT = 128  # what flat plastic facing the viewer comes out as
LABEL = 119  # the flat grey of a label box, as on the Game Boy shells


class Shell:
    def __init__(self, w, h, seed, face=0.0):
        self.w, self.h = w, h
        self.face = face  # the height of its front face, which comes out at FLAT
        self.z = [0.0] * (w * h)  # height of the plastic, in pixels
        self.mask = [0.0] * (w * h)  # how much of each pixel the shell covers
        self.flat = [None] * (w * h)  # a fixed grey or RGB that lighting leaves alone
        self.rng = random.Random(seed)

    # --- shapes -------------------------------------------------------------------------------

    def cover(self, inside, box):
        """Yields (index, coverage) for each pixel `inside(x, y)` touches within box."""
        x0, y0, x1, y1 = box
        for py in range(max(0, int(math.floor(y0))), min(self.h, int(math.ceil(y1)))):
            for px in range(max(0, int(math.floor(x0))), min(self.w, int(math.ceil(x1)))):
                n = 0
                for sy in range(SS):
                    for sx in range(SS):
                        if inside(px + (sx + 0.5) / SS, py + (sy + 0.5) / SS):
                            n += 1
                if n:
                    yield py * self.w + px, n / (SS * SS)

    @staticmethod
    def rrect_inside(x0, y0, x1, y1, radii):
        tl, tr, br, bl = radii

        def inside(x, y):
            if not (x0 <= x < x1 and y0 <= y < y1):
                return False
            for rr, cx, cy, left, top in (
                (tl, x0 + tl, y0 + tl, True, True),
                (tr, x1 - tr, y0 + tr, False, True),
                (br, x1 - br, y1 - br, False, False),
                (bl, x0 + bl, y1 - bl, True, False),
            ):
                if rr > 0 and (x < cx if left else x > cx) and (y < cy if top else y > cy):
                    return (x - cx) ** 2 + (y - cy) ** 2 <= rr * rr
            return True

        return inside

    def outline(self, x0, y0, x1, y1, r=0, radii=None):
        """Adds a box to the shell's outline."""
        inside = self.rrect_inside(x0, y0, x1, y1, radii or (r, r, r, r))
        for i, c in self.cover(inside, (x0, y0, x1, y1)):
            self.mask[i] = max(self.mask[i], c)

    def cut(self, x0, y0, x1, y1):
        """Cuts a box out of the outline."""
        for i, c in self.cover(self.rrect_inside(x0, y0, x1, y1, (0, 0, 0, 0)), (x0, y0, x1, y1)):
            self.mask[i] *= 1 - c

    def height(self, x0, y0, x1, y1, z, r=0, radii=None, add=False):
        """Sets (or with add, raises by) the height of the plastic over a box."""
        inside = self.rrect_inside(x0, y0, x1, y1, radii or (r, r, r, r))
        for i, c in self.cover(inside, (x0, y0, x1, y1)):
            self.z[i] = self.z[i] + z * c if add else self.z[i] * (1 - c) + z * c

    def height_where(self, inside, box, z):
        for i, c in self.cover(inside, box):
            self.z[i] = self.z[i] * (1 - c) + z * c

    def paint(self, x0, y0, x1, y1, colour, r=0, radii=None):
        """Prints a colour over a box, after lighting."""
        inside = self.rrect_inside(x0, y0, x1, y1, radii or (r, r, r, r))
        self.paint_where(inside, (x0, y0, x1, y1), colour)

    def paint_where(self, inside, box, colour):
        if not isinstance(colour, (tuple, list)):
            colour = (colour, colour, colour)
        for i, c in self.cover(inside, box):
            under = self.flat[i]
            if under is None or c < 1:
                self.flat[i] = (colour, c) if under is None else (
                    tuple(u * (1 - c) + v * c for u, v in zip(under[0], colour)),
                    max(under[1], c),
                )
            else:
                self.flat[i] = (colour, 1.0)

    # --- rendering ----------------------------------------------------------------------------

    def blur(self, src, r):
        """Two passes of a separable box blur of radius r."""
        w, h = self.w, self.h
        out = src
        for _ in range(2):
            tmp = [0.0] * (w * h)
            for y in range(h):
                row = out[y * w:(y + 1) * w]
                acc = sum(row[0:min(w, r + 1)]) + row[0] * r
                for x in range(w):
                    tmp[y * w + x] = acc / (2 * r + 1)
                    acc += row[min(w - 1, x + r + 1)] - row[max(0, x - r)]
            res = [0.0] * (w * h)
            for x in range(w):
                col = tmp[x::w]
                acc = sum(col[0:min(h, r + 1)]) + col[0] * r
                for y in range(h):
                    res[y * w + x] = acc / (2 * r + 1)
                    acc += col[min(h - 1, y + r + 1)] - col[max(0, y - r)]
            out = res
        return out

    def render(self, edge=5.0):
        w, h = self.w, self.h
        # The edges of the moulding roll off rather than stop dead.
        solid = [1.0 if m > 0.5 else 0.0 for m in self.mask]
        roll = self.blur(solid, 2)
        z = [zi + edge * (r - 1) for zi, r in zip(self.z, roll)]
        z = self.blur(z, 1)
        far = self.blur(z, 6)
        grain = [self.rng.gauss(0, 1) for _ in range(w * h)]
        soft = self.blur(grain, 1)
        lx, ly, lz = LIGHT
        rgb = []
        for y in range(h):
            for x in range(w):
                i = y * w + x
                gx = (z[i + 1] if x + 1 < w else z[i]) - (z[i - 1] if x else z[i])
                gy = (z[i + w] if y + 1 < h else z[i]) - (z[i - w] if y else z[i])
                nx, ny, nz = -gx * 0.5, -gy * 0.5, 1.0
                n = math.sqrt(nx * nx + ny * ny + nz * nz)
                lit = (nx * lx + ny * ly + nz * lz) / n
                v = FLAT + 120 * (lit - lz)
                v -= 5.0 * max(0.0, far[i] - z[i])  # shade in the bottom of a recess
                v += 2.6 * min(0.0, z[i] - self.face)  # and the deeper, the darker
                v += 5.0 * (0.5 - y / h)  # lit a little more at the top
                v += 4.5 * grain[i] + 3.0 * soft[i]  # the speckle of moulded plastic
                c = (v, v, v)
                fixed = self.flat[i]
                if fixed is not None:
                    colour, cover = fixed
                    tone = colour[0] if colour[0] == colour[1] == colour[2] else None
                    speck = 1.2 * grain[i] if tone is None or tone < 60 else 0.0
                    c = tuple(a * (1 - cover) + (b + speck) * cover for a, b in zip(c, colour))
                rgb.append(c)
        return rgb

    def png(self, path, edge=5.0):
        rgb = self.render(edge)
        raw = bytearray()
        for y in range(self.h):
            raw.append(0)
            for x in range(self.w):
                i = y * self.w + x
                for k in range(3):
                    raw.append(max(0, min(255, round(rgb[i][k]))))
                raw.append(max(0, min(255, round(self.mask[i] * 255))))

        def chunk(kind, data):
            body = kind + data
            return struct.pack(">I", len(data)) + body + struct.pack(">I", zlib.crc32(body))

        with open(path, "wb") as f:
            f.write(b"\x89PNG\r\n\x1a\n")
            f.write(chunk(b"IHDR", struct.pack(">IIBBBBB", self.w, self.h, 8, 6, 0, 0, 0)))
            f.write(chunk(b"IDAT", zlib.compress(bytes(raw), 9)))
            f.write(chunk(b"IEND", b""))


def triangle_edge(pts, width):
    """Whether a point is within width/2 of a triangle's edges."""
    def near(x, y):
        for (ax, ay), (bx, by) in zip(pts, pts[1:] + pts[:1]):
            dx, dy = bx - ax, by - ay
            t = max(0.0, min(1.0, ((x - ax) * dx + (y - ay) * dy) / (dx * dx + dy * dy)))
            if math.hypot(x - ax - t * dx, y - ay - t * dy) <= width / 2:
                return True
        return False

    return near


def snes():
    """An NTSC SNES cart, face on, measured off a photograph of one (798x507 there, scaled to
    360 wide): a raised centre between two ribbed wings, the sticker at the top of the centre,
    a wide recess open to the bottom edge across its lower front with a shallower strip down
    its middle, and a screw low in each wing."""
    W, H = 360, 229
    s = Shell(W, H, seed=2, face=4.0)
    wing_top = 7.7
    body_x0, body_x1 = 57, 304
    # The wings: six slabs a side with deep gaps between them.
    gaps = [42.0, 79.4, 116.8, 154.3, 191.7]
    edges = [wing_top] + gaps + [H]
    for x0, x1, left in ((0, body_x0 + 6, True), (body_x1 - 6, W, False)):
        for k in range(6):
            y0 = edges[k] + (0.9 if k else 0)
            y1 = edges[k + 1] - (0.9 if k < 5 else 0)
            radii = (3, 0, 0, 3) if left else (0, 3, 3, 0)
            s.outline(x0, y0, x1, y1, radii=radii)
            s.height(x0, y0, x1, y1, 0.0)
        for y in gaps:
            s.outline(x0, y - 1.5, x1, y + 1.5)
            s.height(x0, y - 0.9, x1, y + 0.9, -3.5)
    # The raised centre, barely rounded at its corners.
    s.outline(body_x0, 0, body_x1, H, r=3)
    s.height(body_x0, 0, body_x1, H, 4.0, r=3)
    # The recess across the lower front, open to the bottom edge, and the shallower strip down
    # its middle, which stops short of the bottom.
    s.height(67, 138.5, 293, H + 3, 1.0, radii=(3, 3, 0, 0))
    ramp0, ramp1 = 138.5, 200.0
    for k in range(int(ramp1 - ramp0)):
        y = ramp0 + k
        s.height(146, y, 213, y + 1, 4.0 - 3.0 * k / (ramp1 - ramp0))
    # A screw low in each wing.
    for cx in (31.1, 328.4):
        cy = 209.8
        s.height_where(lambda x, y, cx=cx: (x - cx) ** 2 + (y - cy) ** 2 <= 7.2 ** 2,
                       (cx - 8, cy - 8, cx + 8, cy + 8), -3.5)
        s.height_where(lambda x, y, cx=cx: (x - cx) ** 2 + (y - cy) ** 2 <= 5.0 ** 2,
                       (cx - 6, cy - 6, cx + 6, cy + 6), -1.2)
        hexagon = [(cx + 2.4 * math.cos(a * math.pi / 3), cy + 2.4 * math.sin(a * math.pi / 3))
                   for a in range(6)]
        s.height_where(lambda x, y, hx=hexagon: in_polygon(hx, x, y),
                       (cx - 3, cy - 3, cx + 3, cy + 3), -4.0)
    # The sticker, printed after the lighting, black. Down its left: a lozenge, a pointer, a
    # round seal and two lines of small print. The art window (the label box) runs its full
    # height. Down its right: a band along the top, a hatched emblem and a red name bar.
    sx0, sy0, sx1, sy1 = 68, 2.5, 291, 101
    s.paint(sx0, sy0, sx1, sy1, (22, 22, 24), radii=(2, 2, 2, 2))
    s.paint(74, 9, 96, 11, (150, 150, 152))
    s.paint(71, 14, 99.7, 20.3, (206, 40, 46), r=3.1)
    s.paint(72.5, 15.2, 98.2, 19.1, (236, 236, 236), r=1.9)
    s.paint_where(lambda x, y: 43 <= y <= 47 and abs(x - 79) <= (47 - y) * 0.9, (74, 42, 84, 48),
                  (112, 92, 122))
    s.paint_where(lambda x, y: ((x - 85.7) / 11.3) ** 2 + ((y - 77.8) / 7.9) ** 2 <= 1,
                  (74, 69, 98, 87), (186, 196, 120))
    s.paint_where(lambda x, y: ((x - 85.7) / 9.6) ** 2 + ((y - 77.8) / 6.3) ** 2 <= 1,
                  (75, 71, 97, 85), (236, 236, 214))
    for y in (90, 95):
        s.paint(74, y, 98, y + 2, (150, 150, 152))
    s.paint(102, 3, 224, 100, (10, 10, 10))
    s.paint(235, 4, 286, 9, (214, 30, 40))
    emblem = (255, 36, 285.5, 76.7)
    s.paint_where(lambda x, y: emblem[0] <= x < emblem[2] and emblem[1] <= y < emblem[3]
                  and (int(x + y) // 3) % 2 == 0, emblem, (62, 62, 66))
    s.paint(227, 83, 284, 92, (214, 30, 40))
    s.paint(227, 94.5, 284, 99.5, (214, 30, 40))
    return s, (102, 3, 122, 97), (sx0, int(sy0), sx1 - sx0, int(sy1 - sy0) + 1)


def in_polygon(pts, x, y):
    """Whether a point is inside a convex polygon whose corners go round one way."""
    sign = 0
    for (ax, ay), (bx, by) in zip(pts, pts[1:] + pts[:1]):
        cross = (bx - ax) * (y - ay) - (by - ay) * (x - ax)
        if cross:
            if sign and (cross > 0) != (sign > 0):
                return False
            sign = cross
    return True


def nes():
    """An NES cart, face on, measured off a photograph of one (795x896 there, scaled to 304
    wide): the ribbed grip with a pocket at its top, the label in a recess open to the top
    edge, the arrow, a notch in the top edge near each corner, and its lower part stepped in
    by the same amount from both sides, the part that goes into the console."""
    W, H = 304, 343
    s = Shell(W, H, seed=3)
    step, inset = 280, 16.5  # where the lower part steps in, and by how much a side
    s.outline(0, 0, W, H, r=3)
    s.cut(0, step, inset, H)
    s.cut(W - inset, step, W, H)
    s.outline(inset, step - 4, W - inset, H, radii=(0, 0, 3, 3))
    for x0, x1, deep in ((9, 28, 3), (277, 295, 3.5)):
        s.cut(x0, 0, x1, deep)
    # The grip, between a groove either side.
    gx0, gx1 = 47.5, 114
    s.height(gx0 - 1.5, 0, gx0 + 0.5, H, -2.5)
    s.height(gx1 - 0.5, 0, gx1 + 1.8, H, -2.5)
    # The pocket at the top of the grip, its edge a little below the top, three ribs inside.
    pocket = 44
    s.cut(gx0 + 0.5, 0, gx1 - 0.5, 3)
    s.height(gx0 + 1.5, 0, gx1 - 1.5, pocket, -10.0)
    for k in range(3):
        y = 7.5 + k * 13
        s.height(gx0 + 2, y, gx1 - 2, y + 6.0, -7.0)
    # The ribs of the grip: raised bars with narrow grooves between.
    gy0, gy1 = pocket + 1, 306
    ribs = 20
    pitch = (gy1 - gy0) / ribs
    s.height(gx0 + 0.5, gy0, gx1 - 0.5, gy1, 0.8)
    for k in range(ribs + 1):
        y = gy0 + k * pitch
        s.height(gx0 + 0.5, y - 1.2, gx1 - 0.5, y + 1.2, -2.0)
    # The plain panel at the foot of the grip.
    s.height(gx0 + 0.5, gy1 + 2, gx1 - 0.5, H, 0.0)
    s.height(gx0 + 2, gy1 + 3, gx1 - 2, H - 2, -2.2, r=4)
    s.height(gx0 + 4, gy1 + 5, gx1 - 4, H - 4, 0.0, r=3)
    # The label's recess, open to the top edge, and the flat label in it.
    lx0, ly0, lx1, ly1 = 122, 2, 265, 234
    s.cut(lx0, 0, lx1, 1)
    s.height(lx0 - 1.5, 0, lx1 + 1.5, ly1 + 1.5, -2.0, radii=(0, 0, 3, 3))
    s.paint(lx0, ly0, lx1, ly1, LABEL, r=1)
    # The arrow, pressed in, at mid-width.
    mid = W * 0.502
    tri = [(mid - 12.5, 253.0), (mid + 12.5, 253.0), (mid, 277.0)]

    def in_tri(x, y):
        (ax, ay), (bx, by), (cx, cy) = tri
        d1 = (x - bx) * (ay - by) - (ax - bx) * (y - by)
        d2 = (x - cx) * (by - cy) - (bx - cx) * (y - cy)
        d3 = (x - ax) * (cy - ay) - (cx - ax) * (y - ay)
        neg = d1 < 0 or d2 < 0 or d3 < 0
        pos = d1 > 0 or d2 > 0 or d3 > 0
        return not (neg and pos)

    s.height_where(in_tri, (mid - 14, 252, mid + 14, 278), -2.2)
    return s, (lx0, ly0, lx1 - lx0, ly1 - ly0), None


def main():
    root = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "..")
    out = sys.argv[1] if len(sys.argv) > 1 else os.path.join(root, "assets")
    for name, draw in (("snes", snes), ("nes", nes)):
        s, label, sticker = draw()
        s.png(os.path.join(out, f"{name}_neutral.png"))
        print(f"{name}: {s.w}x{s.h} label {label} sticker {sticker}")


if __name__ == "__main__":
    main()
