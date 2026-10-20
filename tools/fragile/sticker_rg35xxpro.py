import math, sys
# The label is as tall as sticker_pro.rs asks: python3 sticker_rg35xxpro.py <height>.
H = int(sys.argv[1]) if len(sys.argv) > 1 else 228
INK = "#f4f4f4"
GREY = "#939396"
def pt(cx, cy, r, deg):
    a = math.radians(deg)
    return cx + r * math.cos(a), cy - r * math.sin(a)
def arc(cx, cy, r, a0, a1):
    """Counter-clockwise arc from a0 to a1 degrees (y up in angle terms)."""
    x0, y0 = pt(cx, cy, r, a0)
    x1, y1 = pt(cx, cy, r, a1)
    large = 1 if (a1 - a0) % 360 > 180 else 0
    return f"M{x0:.2f} {y0:.2f}A{r} {r} 0 {large} 0 {x1:.2f} {y1:.2f}"
def head(cx, cy, r, deg, size, ccw=True):
    """An arrowhead at the end of an arc, pointing along it."""
    x, y = pt(cx, cy, r, deg)
    t = math.radians(deg + (90 if ccw else -90))
    dx, dy = math.cos(t), -math.sin(t)
    nx, ny = -dy, dx
    tip = (x + dx * size, y + dy * size)
    a = (x + nx * size * 0.7, y + ny * size * 0.7)
    b = (x - nx * size * 0.7, y - ny * size * 0.7)
    return f"M{tip[0]:.2f} {tip[1]:.2f}L{a[0]:.2f} {a[1]:.2f}L{b[0]:.2f} {b[1]:.2f}Z"

out = []
w = out.append
w(f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 560 {H}">')
w('  <!-- The RG35XX Pro label: its outline and the marks printed on it, drawn from a photo of')
w('       the device. Made by tools/fragile/sticker_rg35xxpro.py; sticker_pro.rs prints the text. -->')
w(f'  <rect x="1" y="1" width="558" height="{H - 2}" rx="16" ry="16" fill="{GREY}" stroke="#7d7d80" stroke-width="2"/>')
w('  <!-- sticker_pro.rs moves this group to centre the print on the label. -->')
w('  <g id="marks" transform="translate(0 0)">')
w(f'  <g fill="none" stroke="{INK}" stroke-linecap="round" stroke-linejoin="round">')
# Recycle box: a square with three chasing arrows.
w('    <rect x="300" y="86" width="42" height="42" rx="2" stroke-width="2.4"/>')
cx, cy, r = 321, 107, 11
for a0 in (100, 220, 340):
    w(f'    <path d="{arc(cx, cy, r, a0, a0 + 85)}" stroke-width="3.2"/>')
w('  </g>')
w(f'  <g fill="{INK}">')
for a0 in (100, 220, 340):
    w(f'    <path d="{head(cx, cy, r, a0 + 85, 4.2)}"/>')
w('  </g>')
# The row of marks: FC, the crossed bin, CE, the arrow in a disc, the crossed figure.
y = 192
w(f'  <g fill="none" stroke="{INK}" stroke-linecap="butt" stroke-linejoin="miter">')
w(f'    <path d="M30 {y-14}V{y+14}M30 {y-12}H46M30 {y}H42" stroke-width="4"/>')
w(f'    <path d="{arc(58, y, 11, 40, 320)}" stroke-width="4"/>')
w(f'    <path d="{arc(125, y, 12, 60, 300)}" stroke-width="3.6"/>')
w(f'    <path d="{arc(151, y, 12, 60, 300)}M141 {y}H153" stroke-width="3.6"/>')
w('  </g>')
w(f'  <g fill="none" stroke="{INK}" stroke-linecap="round" stroke-width="2.2">')
w(f'    <circle cx="94" cy="{y}" r="14"/>')
w(f'    <path d="M88 {y-5}H100L98 {y+9}H90ZM86 {y-8}H102M84 {y-10}L104 {y+10}"/>')
w(f'    <circle cx="258" cy="156" r="11"/>')
w(f'    <path d="M250 148L266 164M255 151V158M255 154L259 151M262 158V163"/>')
w('  </g>')
w(f'  <circle cx="192" cy="{y}" r="14" fill="{INK}"/>')
w(f'  <path d="{arc(192, y, 8, 120, 400)}" fill="none" stroke="{GREY}" stroke-width="3"/>')
w(f'  <path d="{head(192, y, 8, 400, 4)}" fill="{GREY}"/>')
w('  </g>')
w('</svg>')
sys.stdout.write("\n".join(out) + "\n")
