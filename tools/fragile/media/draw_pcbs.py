#!/usr/bin/env python3
"""Draws inside PCBs, interiors, and data-side disc art for NES, SNES, and PlayStation.

Outputs in OUT_DIR (default: assets/):
- nes_board.png, nes_interior.png (304x343)
- snes_board.png, snes_interior.png (360x229)
- disc_board.png (337x337)

Pure Python, no dependencies:
    python3 tools/fragile/media/draw_pcbs.py [OUT_DIR]
"""

import math
import os
import random
import struct
import sys
import zlib

# Palette matching assets/nds_board.png and assets/gb_board.png
PCB_BASE = (14, 88, 52)
PCB_MOTTLE = (28, 108, 66)
TRACE = (40, 138, 84)
SILK = (224, 228, 220)
CHIP = (22, 26, 34)
CHIP_HI = (36, 40, 50)
LEG = (198, 200, 196)
GOLD = (204, 166, 62)
GOLD_HI = (232, 198, 96)
GOLD_SH = (150, 118, 36)
SOLDER = (182, 186, 188)
COIN_SIL = (212, 216, 222)
COIN_RIM = (156, 160, 168)
CAP_TAN = (186, 116, 48)
CAP_BLUE = (48, 92, 168)


class Canvas:
    def __init__(self, w, h, seed=1):
        self.w, self.h = w, h
        self.rng = random.Random(seed)
        self.px = [[0.0, 0.0, 0.0, 0.0] for _ in range(w * h)]

    def blend(self, x, y, rgb, a=1.0):
        if a <= 0.0 or x < 0 or y < 0 or x >= self.w or y >= self.h:
            return
        i = y * self.w + x
        p = self.px[i]
        a = min(1.0, max(0.0, a))
        out_a = a + p[3] * (1.0 - a)
        if out_a <= 1e-6:
            return
        for k in range(3):
            p[k] = (rgb[k] * a + p[k] * p[3] * (1.0 - a)) / out_a
        p[3] = out_a

    def fill_rrect(self, x0, y0, x1, y1, r, rgb, a=1.0):
        ix0, iy0 = max(0, int(math.floor(x0))), max(0, int(math.floor(y0)))
        ix1, iy1 = min(self.w, int(math.ceil(x1))), min(self.h, int(math.ceil(y1)))
        for py in range(iy0, iy1):
            cy_u = py + 0.5
            for px in range(ix0, ix1):
                cx_u = px + 0.5
                if not (x0 <= cx_u <= x1 and y0 <= cy_u <= y1):
                    # soft edge check
                    pass
                cov = 0
                for sy in (0.25, 0.75):
                    y = py + sy
                    if not (y0 <= y < y1):
                        continue
                    for sx in (0.25, 0.75):
                        x = px + sx
                        if not (x0 <= x < x1):
                            continue
                        ok = True
                        if r > 0:
                            for cx, cy, left, top in (
                                (x0 + r, y0 + r, True, True),
                                (x1 - r, y0 + r, False, True),
                                (x1 - r, y1 - r, False, False),
                                (x0 + r, y1 - r, True, False),
                            ):
                                if (x < cx if left else x > cx) and (y < cy if top else y > cy):
                                    if (x - cx) ** 2 + (y - cy) ** 2 > r * r:
                                        ok = False
                                        break
                        if ok:
                            cov += 1
                if cov:
                    self.blend(px, py, rgb, a * (cov / 4.0))

    def fill_circle(self, cx, cy, r, rgb, a=1.0):
        ix0, iy0 = max(0, int(math.floor(cx - r - 1))), max(0, int(math.floor(cy - r - 1)))
        ix1, iy1 = min(self.w, int(math.ceil(cx + r + 1))), min(self.h, int(math.ceil(cy + r + 1)))
        r2 = r * r
        for py in range(iy0, iy1):
            for px in range(ix0, ix1):
                cov = 0
                for sy in (0.25, 0.75):
                    dy = (py + sy) - cy
                    for sx in (0.25, 0.75):
                        dx = (px + sx) - cx
                        if dx * dx + dy * dy <= r2:
                            cov += 1
                if cov:
                    self.blend(px, py, rgb, a * (cov / 4.0))

    def ring_circle(self, cx, cy, r_in, r_out, rgb, a=1.0):
        ix0, iy0 = max(0, int(math.floor(cx - r_out - 1))), max(0, int(math.floor(cy - r_out - 1)))
        ix1, iy1 = min(self.w, int(math.ceil(cx + r_out + 1))), min(self.h, int(math.ceil(cy + r_out + 1)))
        r_in2, r_out2 = r_in * r_in, r_out * r_out
        for py in range(iy0, iy1):
            for px in range(ix0, ix1):
                cov = 0
                for sy in (0.25, 0.75):
                    dy = (py + sy) - cy
                    for sx in (0.25, 0.75):
                        dx = (px + sx) - cx
                        d2 = dx * dx + dy * dy
                        if r_in2 <= d2 <= r_out2:
                            cov += 1
                if cov:
                    self.blend(px, py, rgb, a * (cov / 4.0))

    def line(self, x0, y0, x1, y1, width, rgb, a=1.0):
        hw = width * 0.5
        bx0 = max(0, int(math.floor(min(x0, x1) - hw - 1)))
        bx1 = min(self.w, int(math.ceil(max(x0, x1) + hw + 1)))
        by0 = max(0, int(math.floor(min(y0, y1) - hw - 1)))
        by1 = min(self.h, int(math.ceil(max(y0, y1) + hw + 1)))
        dx, dy = x1 - x0, y1 - y0
        l2 = dx * dx + dy * dy
        for py in range(by0, by1):
            for px in range(bx0, bx1):
                x, y = px + 0.5, py + 0.5
                t = 0.0 if l2 == 0 else max(0.0, min(1.0, ((x - x0) * dx + (y - y0) * dy) / l2))
                d = math.hypot(x - (x0 + t * dx), y - (y0 + t * dy))
                cov = max(0.0, min(1.0, hw + 0.5 - d))
                if cov > 0.0:
                    self.blend(px, py, rgb, a * cov)

    def pcb_grain_traces(self, x0, y0, x1, y1, n_traces=34):
        """Adds organic copper-plane mottling and orthogonal/45-deg traces inside PCB region."""
        for py in range(max(0, int(y0)), min(self.h, int(y1))):
            for px in range(max(0, int(x0)), min(self.w, int(x1))):
                i = py * self.w + px
                if self.px[i][3] < 0.1:
                    continue
                n = math.sin(px * 0.09 + py * 0.04) * math.cos(py * 0.08 - px * 0.03)
                g = self.rng.uniform(-4.5, 4.5) + n * 7.0
                for k in range(3):
                    base = PCB_BASE[k] + (PCB_MOTTLE[k] - PCB_BASE[k]) * max(0.0, n)
                    self.px[i][k] = max(0.0, min(255.0, base + g))
        for _ in range(n_traces):
            cx = self.rng.uniform(x0 + 10, x1 - 10)
            cy = self.rng.uniform(y0 + 10, y1 - 36)
            for _ in range(self.rng.randint(2, 4)):
                step = self.rng.choice([(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (-1, 1)])
                dist = self.rng.uniform(14, 48)
                nx = max(x0 + 6, min(x1 - 6, cx + step[0] * dist))
                ny = max(y0 + 6, min(y1 - 30, cy + step[1] * dist))
                self.line(cx, cy, nx, ny, 1.5, TRACE, 0.85)
                cx, cy = nx, ny
            self.fill_circle(cx, cy, 2.1, GOLD, 0.9)
            self.fill_circle(cx, cy, 0.9, (12, 34, 22), 1.0)

    def chip_dip(self, x0, y0, x1, y1, pins_h=True, pin_pitch=6.0):
        """A matte black DIP or SOIC IC package with metallic pins."""
        self.fill_rrect(x0 + 2, y0 + 2, x1 + 2, y1 + 2, 2, (8, 12, 10), 0.45)
        if pins_h:
            y = y0 + pin_pitch * 0.7
            while y < y1 - pin_pitch * 0.4:
                self.fill_rrect(x0 - 4, y - 1.3, x0 + 1, y + 1.3, 0.4, LEG)
                self.fill_rrect(x1 - 1, y - 1.3, x1 + 4, y + 1.3, 0.4, LEG)
                y += pin_pitch
        else:
            x = x0 + pin_pitch * 0.7
            while x < x1 - pin_pitch * 0.4:
                self.fill_rrect(x - 1.3, y0 - 4, x + 1.3, y0 + 1, 0.4, LEG)
                self.fill_rrect(x - 1.3, y1 - 1, x + 1.3, y1 + 4, 0.4, LEG)
                x += pin_pitch
        self.fill_rrect(x0, y0, x1, y1, 2.2, CHIP)
        self.fill_rrect(x0 + 2, y0 + 2, x1 - 2, y0 + 4.5, 1.0, CHIP_HI, 0.6)
        self.fill_circle(x0 + 7, y0 + 7, 2.2, (46, 52, 64), 0.9)

    def chip_qfp(self, x0, y0, x1, y1, pitch=3.2):
        """A square surface-mount quad flat pack mapper chip."""
        self.fill_rrect(x0 + 2, y0 + 2, x1 + 2, y1 + 2, 2, (8, 12, 10), 0.45)
        x = x0 + 3.0
        while x < x1 - 2.5:
            self.fill_rrect(x, y0 - 3.5, x + 1.4, y0 + 0.5, 0.2, LEG)
            self.fill_rrect(x, y1 - 0.5, x + 1.4, y1 + 3.5, 0.2, LEG)
            x += pitch
        y = y0 + 3.0
        while y < y1 - 2.5:
            self.fill_rrect(x0 - 3.5, y, x0 + 0.5, y + 1.4, 0.2, LEG)
            self.fill_rrect(x1 - 0.5, y, x1 + 3.5, y + 1.4, 0.2, LEG)
            y += pitch
        self.fill_rrect(x0, y0, x1, y1, 2.0, CHIP)
        self.fill_rrect(x0 + 2, y0 + 2, x1 - 2, y0 + 4.0, 1.0, CHIP_HI, 0.5)
        self.fill_circle(x0 + 6, y0 + 6, 2.0, (56, 62, 74))

    def battery_cr2032(self, cx, cy, r=19.0):
        """Circular CR2032 coin battery in solder-tabbed holder."""
        self.fill_circle(cx + 1.5, cy + 2.0, r + 2.5, (8, 12, 10), 0.4)
        self.fill_rrect(cx - r - 5, cy - 4, cx + r + 5, cy + 4, 2, GOLD_SH)
        self.fill_circle(cx, cy, r + 1.5, COIN_RIM)
        self.fill_circle(cx, cy, r - 0.5, COIN_SIL)
        self.ring_circle(cx, cy, r - 3.0, r - 1.5, (186, 190, 198))
        self.fill_rrect(cx - 5, cy - r - 2, cx + 5, cy + 3, 1.5, (230, 234, 240))
        self.line(cx - 4, cy - 10, cx + 4, cy - 10, 1.8, (140, 144, 152))
        self.line(cx, cy - 14, cx, cy - 6, 1.8, (140, 144, 152))

    def edge_fingers(self, x0, x1, y0, y1, count, bevel=True):
        """Gold edge connector contacts along the board foot."""
        self.fill_rrect(x0 - 4, y0 - 3, x1 + 4, y1, 2, (12, 68, 40), 0.7)
        pitch = (x1 - x0) / count
        fw = pitch * 0.58
        for k in range(count):
            fx = x0 + pitch * k + (pitch - fw) * 0.5
            self.fill_rrect(fx, y0, fx + fw, y1, 1.0, GOLD)
            self.fill_rrect(fx + 0.3, y0 + 0.5, fx + fw * 0.45, y1 - 2, 0.5, GOLD_HI, 0.55)
            self.fill_rrect(fx, y1 - 3, fx + fw, y1, 0.5, GOLD_SH, 0.65)
            self.fill_circle(fx + fw * 0.5, y0 - 4.5, 1.1, GOLD_HI, 0.85)

    def write_png(self, path):
        raw = bytearray()
        for y in range(self.h):
            raw.append(0)
            row = y * self.w
            for x in range(self.w):
                r, g, b, a = self.px[row + x]
                raw.append(max(0, min(255, round(r))))
                raw.append(max(0, min(255, round(g))))
                raw.append(max(0, min(255, round(b))))
                raw.append(max(0, min(255, round(a * 255.0))))

        def chunk(kind, data):
            body = kind + data
            return struct.pack(">I", len(data)) + body + struct.pack(">I", zlib.crc32(body))

        with open(path, "wb") as f:
            f.write(b"\x89PNG\r\n\x1a\n")
            f.write(chunk(b"IHDR", struct.pack(">IIBBBBB", self.w, self.h, 8, 6, 0, 0, 0)))
            f.write(chunk(b"IDAT", zlib.compress(bytes(raw), 9)))
            f.write(chunk(b"IEND", b""))


def draw_nes(out_dir):
    """NES cart 304x343: grey inner shell cavity (nes_interior) + MMC/NROM PCB (nes_board)."""
    W, H = 304, 343
    # 1. nes_interior.png: inner grey shell cavity & guide ribs visible behind/around PCB
    shell = Canvas(W, H, seed=101)
    shell.fill_rrect(4, 4, W - 4, 282, 4, (54, 54, 58))
    shell.fill_rrect(20, 276, W - 20, H - 3, 3, (54, 54, 58))
    shell.fill_rrect(12, 12, W - 12, 274, 3, (36, 36, 40))
    shell.fill_rrect(26, 270, W - 26, H - 6, 2, (36, 36, 40))
    # Internal vertical plastic structural ribs & 3 screw pillars
    for rx in (44, 96, 152, 208, 256):
        shell.fill_rrect(rx - 2, 14, rx + 2, 110, 1, (48, 48, 52))
    for sx, sy in ((28, 30), (276, 30), (152, 30), (36, 250), (268, 250)):
        shell.fill_circle(sx, sy, 8.5, (64, 64, 68))
        shell.fill_circle(sx, sy, 4.0, (24, 24, 28))
    shell.write_png(os.path.join(out_dir, "nes_interior.png"))

    # 2. nes_board.png: full NES PCB sitting inside lower/mid shell, exposing PRG & CHR ROMs
    pcb = Canvas(W, H, seed=102)
    bx0, by0, bx1, by1 = 24, 92, 280, 336
    pcb.fill_rrect(bx0, by0, bx1, 282, 4, PCB_BASE)
    pcb.fill_rrect(30, 278, W - 30, by1, 2, PCB_BASE)
    pcb.pcb_grain_traces(bx0, by0, bx1, by1, n_traces=42)
    # Screw notches along sides
    for ny in (132, 244):
        pcb.fill_circle(bx0 + 8, ny, 5.5, (14, 18, 16))
        pcb.fill_circle(bx1 - 8, ny, 5.5, (14, 18, 16))
    # Silkscreen boxes
    pcb.fill_rrect(48, 138, 148, 222, 2, SILK, 0.14)
    pcb.fill_rrect(156, 138, 256, 222, 2, SILK, 0.14)
    # ROM chips: PRG-ROM at (52, 145, 92, 70) -> x:52..144, y:145..215 (text target!)
    pcb.chip_dip(52, 145, 144, 215, pins_h=True, pin_pitch=5.0)
    # CHR-ROM at right
    pcb.chip_dip(160, 145, 252, 215, pins_h=True, pin_pitch=5.0)
    # MMC3 mapper QFP chip centered below ROMs
    pcb.chip_qfp(126, 232, 178, 282, pitch=3.2)
    # CIC lockout chip (left) + CR2032 save battery (top right) + decoupling caps
    pcb.chip_dip(52, 238, 86, 278, pins_h=True, pin_pitch=5.0)
    pcb.battery_cr2032(226, 114, r=17.5)
    pcb.fill_circle(70, 114, 7.5, CAP_BLUE)
    pcb.fill_circle(70, 114, 5.0, (70, 118, 198))
    pcb.fill_rrect(94, 110, 108, 118, 2, CAP_TAN)
    pcb.fill_rrect(188, 244, 206, 260, 2, CAP_TAN)
    # 36 gold edge contacts along stepped-in bottom
    pcb.edge_fingers(34, W - 34, 304, by1 - 2, 36)
    pcb.write_png(os.path.join(out_dir, "nes_board.png"))


def draw_snes(out_dir):
    """SNES cart 360x229: inner wings cavity (snes_interior) + SHVC PCB (snes_board)."""
    W, H = 360, 229
    # 1. snes_interior.png: matte interior shell cavity matching ribbed wings shape
    shell = Canvas(W, H, seed=201)
    shell.fill_rrect(4, 10, 60, H - 3, 3, (56, 56, 62))
    shell.fill_rrect(W - 60, 10, W - 4, H - 3, 3, (56, 56, 62))
    shell.fill_rrect(58, 4, W - 58, H - 3, 3, (56, 56, 62))
    shell.fill_rrect(64, 12, W - 64, H - 6, 3, (36, 36, 40))
    for rx in (100, 140, 180, 220, 260):
        shell.fill_rrect(rx - 1.5, 14, rx + 1.5, 58, 1, (48, 48, 52))
    for sx, sy in ((32, 208), (328, 208)):
        shell.fill_circle(sx, sy, 7.5, (68, 68, 72))
        shell.fill_circle(sx, sy, 3.8, (24, 24, 28))
    shell.write_png(os.path.join(out_dir, "snes_interior.png"))

    # 2. snes_board.png: wide horizontal SHVC PCB with Mask ROM + SRAM + CR2032 + 44 edge pins
    pcb = Canvas(W, H, seed=202)
    bx0, by0, bx1, by1 = 68, 28, 292, 224
    pcb.fill_rrect(bx0, by0, bx1, by1, 4, PCB_BASE)
    pcb.pcb_grain_traces(bx0, by0, bx1, by1, n_traces=36)
    # Silkscreen outlines for Mask ROM + twin core sockets
    pcb.fill_rrect(112, 58, 248, 122, 2, SILK, 0.14)
    pcb.fill_rrect(79, 133, 145, 181, 2, SILK, 0.12)
    pcb.fill_rrect(153, 133, 219, 181, 2, SILK, 0.12)
    # Main Mask ROM DIP horizontal at (116, 62, 128, 56) -> x:116..244, y:62..118 (text target!)
    pcb.chip_dip(116, 62, 244, 118, pins_h=False, pin_pitch=6.0)
    # CR2032 save battery on right (clear of both sockets)
    pcb.battery_cr2032(258, 154, r=17.5)
    # Decoupling caps & crystal
    pcb.fill_circle(92, 86, 7.0, CAP_BLUE)
    pcb.fill_circle(92, 86, 4.5, (70, 118, 198))
    pcb.fill_rrect(252, 74, 272, 92, 3, (206, 210, 216))  # silver resonator crystal
    pcb.fill_rrect(80, 42, 98, 49, 2, CAP_TAN)
    # 44 gold edge contacts along bottom edge
    pcb.edge_fingers(76, 284, 196, by1 - 1, 44)
    pcb.write_png(os.path.join(out_dir, "snes_board.png"))


def draw_disc(out_dir):
    """PlayStation data-side silver CD-ROM (337x337), aligned to disc_neutral.png."""
    W, H = 337, 337
    cx, cy = 168.5, 168.5
    r_hole = 21.5
    r_clamp_in = 22.0
    r_mirror_in = 57.5
    r_outer = 168.0
    img = Canvas(W, H, seed=301)

    for py in range(H):
        for px in range(W):
            dx = (px + 0.5) - cx
            dy = (py + 0.5) - cy
            r = math.hypot(dx, dy)
            if r < r_hole or r > r_outer:
                continue
            # Edge anti-aliasing
            a = 1.0
            if r < r_hole + 1.0:
                a = max(0.0, min(1.0, r - r_hole))
            elif r > r_outer - 1.0:
                a = max(0.0, min(1.0, r_outer - r))

            ang = math.atan2(dy, dx)
            # Outer polycarbonate bevel & inner hub ring
            if r < r_mirror_in:
                # Translucent polycarbonate clamping ring & stacking ridge
                base = 172.0 - 16.0 * math.cos((r - 22.0) * 0.25)
                if 41.0 <= r <= 45.0:
                    base += 28.0  # stacking ring highlight
                if 54.0 <= r <= 57.5:
                    base -= 35.0  # inner mirror seam
                # Subtle radial barcode dashes in inner clamping band (48..54)
                if 48.0 <= r <= 53.5 and ((int((ang + math.pi) * 28)) % 3 == 0):
                    base -= 22.0
                col = (base, base + 2, base + 5)
            else:
                # Silver reflective data surface with subtle concentric track sheen
                track = 3.2 * math.sin(r * 1.4) + 1.5 * math.sin(r * 0.35)
                # Diagonal optical highlight (matches top-left studio lighting)
                spec = math.exp(-((math.sin(ang + 0.78)) ** 2) * 14.0) * 34.0
                spec2 = math.exp(-((math.cos(ang + 0.78)) ** 2) * 24.0) * 18.0
                # Soft CD diffraction rainbow shimmer along light bands
                bow = math.exp(-((math.sin(ang + 0.78)) ** 2) * 8.0) * 24.0
                phase = ang * 2.0 + r * 0.045
                rb = 202.0 + track + spec + spec2 + bow * math.sin(phase)
                gb = 205.0 + track + spec + spec2 + bow * math.sin(phase + 2.1)
                bb = 212.0 + track + spec + spec2 + bow * math.sin(phase + 4.2)
                if r > r_outer - 3.0:
                    shade = (r_outer - r) / 3.0
                    rb *= 0.75 + 0.25 * shade
                    gb *= 0.75 + 0.25 * shade
                    bb *= 0.75 + 0.25 * shade
                col = (rb, gb, bb)

            i = py * W + px
            img.px[i] = [max(0.0, min(255.0, col[0])),
                         max(0.0, min(255.0, col[1])),
                         max(0.0, min(255.0, col[2])),
                         a]

    img.write_png(os.path.join(out_dir, "disc_board.png"))


def main():
    root = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "..")
    out = sys.argv[1] if len(sys.argv) > 1 else os.path.join(root, "assets")
    draw_nes(out)
    draw_snes(out)
    draw_disc(out)
    print("Generated nes_board/nes_interior (304x343), snes_board/snes_interior (360x229), disc_board (337x337)")


if __name__ == "__main__":
    main()
