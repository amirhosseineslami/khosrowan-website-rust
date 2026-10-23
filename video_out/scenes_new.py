#!/usr/bin/env python3
"""Never Beg v3 — semantic, beat-synced motion engine.

The narration is split into sentences; every sentence is tagged with the
concept it advances, and a dedicated shape *for that idea* animates in at the
exact moment the line is spoken. Ambient motion (drifting embers, breathing
light) runs every single frame so nothing is ever static.

Design rules honoured:
  * shapes transfer the spoken idea in that exact moment
  * continuous motion throughout every section, not just the opening
  * no per-letter boxes, no scene-to-scene fades, one lead element
"""
import math
import os
import numpy as np
from PIL import Image
from motion_engine import (Canvas, KF, W, H, FPS,
                           ease_out_back, ease_in_out_cubic, ease_in_out_sine,
                           ease_out_cubic, ease_out_bounce, pulse)

TEAL = (40, 120, 140)
ORANGE = (230, 150, 70)
AMBER = (255, 190, 90)
GREEN = (90, 200, 130)
RED = (220, 70, 60)
WHITE = (235, 238, 244)
GREY = (120, 128, 142)
GOLD = (220, 190, 130)
BLUE = (90, 160, 210)
DARK = (8, 10, 16)

# --------------------------------------------------------------------------
# Scheduler: distribute sentences over a section by relative word count.
# --------------------------------------------------------------------------
def _schedule(sentences, sec_dur, gap=0.22):
    words = [max(1, len(s.split())) for s in sentences]
    total = sum(words)
    n = len(sentences)
    gaps = gap * (n - 1)
    spoken = max(0.1, sec_dur - gaps)
    unit = spoken / total
    out, t = [], 0.0
    for w in words:
        d = w * unit
        out.append((t, t + d))
        t += d + gap
    out[-1] = (out[-1][0], sec_dur)
    return out


# --------------------------------------------------------------------------
# Atmosphere — always on.
# --------------------------------------------------------------------------
_PAL = {
    "cool": [(8, 12, 22), (18, 28, 46), (6, 9, 18)],
    "amber": [(24, 18, 12), (44, 34, 22), (10, 9, 10)],
    "red": [(28, 10, 9), (16, 8, 9), (6, 6, 10)],
    "blue": [(7, 10, 18), (16, 22, 38), (5, 7, 12)],
    "gold": [(26, 22, 14), (40, 34, 22), (10, 10, 10)],
}

def _atmosphere(c, t, mood, base=True):
    stops = _PAL[mood]
    breath = 1.0 + 0.04 * math.sin(t * 0.5)
    if base:
        scaled = [(i / max(1, len(stops) - 1),
                   tuple(min(255, int(v * breath)) for v in col))
                  for i, col in enumerate(stops)]
        c.base_gradient(scaled)
    for i in range(70):
        ph = (i * 12.9898) % 1.0
        spd = 0.015 + (i * 0.013) % 0.05
        drift = (i * 17.31) % 6.28
        y = (ph * H - t * 30 * spd) % H
        x = (W * ((i * 0.61803) % 1.0) + math.sin(t * 0.3 + drift) * 40)
        r = 2 + (i % 4)
        a = 0.05 + 0.05 * (0.5 + 0.5 * math.sin(t * 1.2 + i))
        c.glow(x, y, r, a, (255, 220, 180) if mood == "amber" else (150, 180, 220))


# --------------------------------------------------------------------------
# Cinematic backdrop: the hero image for each section, animated with a slow
# Ken-Burns drift so it never sits still.
# --------------------------------------------------------------------------
_BACKCACHE = {}

def _backdrop(sid):
    """Return cached (arr, nw, nh) backdrop for a section, or None."""
    if sid in _BACKCACHE:
        return _BACKCACHE[sid]
    p = f"img/{sid}.png"
    if not os.path.exists(p):
        _BACKCACHE[sid] = None
        return None
    im = Image.open(p).convert("RGB")
    # overscan so we have slack for both pan axes after the 16:9 crop
    scale = max(2240 / im.width, 1260 / im.height)
    nw, nh = int(im.width * scale) + 8, int(im.height * scale) + 8
    im = im.resize((nw, nh), Image.LANCZOS)
    arr = np.asarray(im, dtype=np.float32)
    _BACKCACHE[sid] = (arr, nw, nh)
    return arr, nw, nh


def _draw_backdrop(c, sid, t):
    bd = _backdrop(sid)
    if bd is None:
        return False
    arr, nw, nh = bd
    z = 1.0 + 0.09 * (0.5 + 0.5 * math.sin(t * 0.15))
    ww = int(1920 / z)
    wh = int(1080 / z)
    cx = nw / 2 + 130 * math.sin(t * 0.2)
    cy = nh / 2 + 100 * math.cos(t * 0.17)
    x0 = int(max(0, min(nw - ww, cx - ww / 2)))
    y0 = int(max(0, min(nh - wh, cy - wh / 2)))
    crop = arr[y0:y0 + wh, x0:x0 + ww]
    if crop.shape[0] != 1080 or crop.shape[1] != 1920:
        crop = np.asarray(
            Image.fromarray(crop.astype(np.uint8)).resize(
                (1920, 1080), Image.BILINEAR), dtype=np.float32)
    c.img[:] = crop * 0.55
    return True


# --------------------------------------------------------------------------
# Generic helpers
# --------------------------------------------------------------------------
def _fade_in(t, dur=0.6):
    return min(1.0, t / dur)

def _card(c, cx, cy, w, h, k, title, sub, tcol=AMBER):
    e = ease_out_back(k)
    yy = cy + (1 - e) * 80
    c.rect((cx - w / 2, yy - h / 2, cx + w / 2, yy + h / 2),
           (14, 20, 34), width=2, round_=18)
    c.text((cx, yy - 12), title, 38, tcol, alpha=e)
    if sub:
        c.text((cx, yy + 28), sub, 24, GREY, kind="reg", alpha=e)


def _badge(c, cx, cy, radius, text, k, col=AMBER, textcol=(20, 20, 30)):
    e = ease_out_back(k)
    r = radius * e
    c.glow(cx, cy, r * 1.6, 0.25 * e, col)
    # ring
    pts = []
    for i in range(40):
        a = i / 40 * 2 * math.pi
        pts.append((cx + r * math.cos(a), cy + r * math.sin(a)))
    c.line(cx + r, cy, cx + r, cy, col, width=0)  # noop to keep signature
    c.glow(cx, cy, r, 0.55 * e, col)
    c.text((cx, cy), text, int(r * 0.9), textcol, alpha=e)


def _up_arrow(c, x, y, k, col=GREEN, label=None):
    h = 160 * k
    c.line(x, y, x, y - h, col, width=12)
    c.poly([(x - 30, y - h + 40), (x + 30, y - h + 40), (x, y - h - 30)],
           col)
    if label:
        c.text((x + 50, y - h / 2), label, 30, col, anchor="lm")


# --------------------------------------------------------------------------
# Semantic motifs — each draws ONE idea.
# --------------------------------------------------------------------------
def _lead(c, t, title, sub=None):
    c.text((W / 2, 150), title, 84, AMBER, tracking=18,
           alpha=_fade_in(t))
    if sub:
        c.text((W / 2, 232), sub, 32, GREY, kind="reg",
               alpha=_fade_in(t - 0.5, 0.6))


def m_walkout(c, t, k):
    # a woman walks out through a door of light
    g = ease_out_cubic(k)
    c.glow(1510, 560, 420 * (0.4 + 0.6 * g), 0.55, AMBER)
    c.glow(1510, 560, 180, 0.5 * (0.5 + 0.5 * g), (255, 215, 140))
    fx = 900 + ease_in_out_cubic(g) * 400
    fh = 150 * (1 + 0.04 * math.sin(t * 1.3))
    c.poly([(fx - 24, 640), (fx + 24, 640), (fx + 32, 640 + fh),
            (fx, 640 + fh * 0.88), (fx - 32, 640 + fh)], (9, 12, 20), 4)
    c.text((W / 2, 150), "NEVER BEG", 96, AMBER, tracking=24,
           alpha=_fade_in(t, 0.8))


def m_count3(c, t, k):
    items = ["no chasing", "no over-explaining", "exit open"]
    for i, it in enumerate(items):
        y = 420 + i * 110
        if t > 0.3 + i * 0.6:
            kk = min(1.0, (t - (0.3 + i * 0.6)) / 0.7)
            _card(c, W / 2, y, 520, 80, kk, it, None)


def m_gauge_near(c, t, k):
    _lead(c, t, "SHE'S NEVER 100% SURE")
    # gauge arc near full but not 100
    e = ease_in_out_cubic(min(1.0, k * 1.4))
    frac = 0.92 * e
    cx, cy = 960, 560
    c.glow(cx, cy, 300, 0.12, TEAL)
    # background arc
    for i in range(120):
        a = math.pi + (i / 120) * math.pi
        x0 = cx + 260 * math.cos(a); y0 = cy + 260 * math.sin(a)
        c.glow(x0, y0, 4, 0.05, GREY)
    # fill arc
    for i in range(int(120 * frac)):
        a = math.pi + (i / 120) * math.pi
        x0 = cx + 260 * math.cos(a); y0 = cy + 260 * math.sin(a)
        c.glow(x0, y0, 6, 0.5, ORANGE)
    c.text((cx, cy), f"{int(frac*100)}%", 72, WHITE)
    c.text((cx, cy + 80), "not 100%", 30, GREY, kind="reg")
    c.text((cx - 300, cy + 220), "doubt", 26, GREY, kind="reg")
    c.text((cx + 300, cy + 220), "certainty", 26, GREY, kind="reg")


def m_grass(c, t, k):
    _lead(c, t, "THE GRASS IS GREENER")
    # fence line + green patch growing on the horizon
    e = ease_in_out_sine(min(1.0, k * 1.5))
    # fence
    for x in range(480, 1500, 90):
        c.line(x, 600, x, 760, (40, 46, 62), width=6)
    c.line(480, 680, 1500, 680, (40, 46, 62), width=6)
    # green grass beyond
    gw = 900 * e
    c.rect((520, 640, 520 + gw, 680), GREEN, round_=0)
    c.text((960, 820), "her theory: better over there", 30, WHITE,
           kind="reg", alpha=e)


def m_market(c, t, k):
    _lead(c, t, "HER MARKET VALUE: HIGHER")
    e = ease_in_out_cubic(min(1.0, k * 1.4))
    # rising bar
    for i in range(6):
        bh = (60 + i * 34) * e
        x = 620 + i * 120
        c.rect((x, 720 - bh, x + 70, 720), (30, 60, 90), round_=8)
        c.text((x + 35, 720 - bh - 30), f"+{i*2}", 24, GREEN,
               anchor="mm" if bh > 50 else "mb")
    c.text((960, 850), "market value", 30, WHITE, kind="reg", alpha=e)
    _up_arrow(c, 1640, 620, e, col=GREEN)


def m_experiment(c, t, k):
    _lead(c, t, "SHE RUNS AN EXPERIMENT")
    e = ease_in_out_cubic(min(1.0, k * 1.4))
    # beaker
    bx, by = 960, 620
    c.rect((bx - 110, by - 200, bx + 110, by + 60), (16, 22, 36), width=3,
           round_=10)
    c.line(bx - 110, by - 200, bx - 60, by - 260, (16, 22, 36), width=8)
    c.line(bx + 110, by - 200, bx + 60, by - 260, (16, 22, 36), width=8)
    # liquid
    c.rect((bx - 100, by - 40, bx + 100, by + 50), (40, 90, 140), round_=8)
    # bubbles
    for i in range(5):
        bu = (t * 0.5 + i * 0.2) % 1.0
        c.glow(bx - 60 + i * 30, by - 40 - bu * 120, 6, 0.6 * (1 - bu), TEAL)
    c.text((bx + 170, by), "can I do better?", 30, WHITE, kind="reg",
           alpha=e)


def m_chase_confirm(c, t, k):
    _lead(c, t, "YOU CHASE -> FREE CONFIRMATION")
    e = ease_in_out_cubic(min(1.0, k * 1.3))
    # chaser behind
    x1 = 400 + e * 300
    fh = 130
    c.poly([(x1 - 20, 700), (x1 + 20, 700), (x1 + 26, 700 + fh),
            (x1, 700 + fh * 0.88), (x1 - 26, 700 + fh)], (40, 46, 60), 3)
    # her ahead
    x2 = 1300 + (1 - e) * 200
    c.poly([(x2 - 22, 700), (x2 + 22, 700), (x2 + 28, 700 + fh),
            (x2, 700 + fh * 0.88), (x2 - 28, 700 + fh)], (150, 110, 90), 3)
    # red CONFIRMED stamp slams on her theory
    if e > 0.5:
        sk = min(1.0, (e - 0.5) / 0.35)
        rot = 0.5
        cx, cy = 960, 360
        w, h = 340, 120
        # stamp box
        pts = []
        cos, sin = math.cos(rot), math.sin(rot)
        for dx, dy in [(-w/2, -h/2), (w/2, -h/2), (w/2, h/2), (-w/2, h/2)]:
            pts.append((cx + dx*cos - dy*sin, cy + dx*sin + dy*cos))
        c.poly(pts, (40, 12, 10))
        c.text((cx, cy), "CONFIRMED", 40, RED, alpha=sk)


def m_value_spot(c, t, k):
    _lead(c, t, "WHAT SHE'S WALKING AWAY FROM STILL HAS VALUE")
    e = ease_in_out_sine(min(1.0, k * 1.4))
    # spotlight on a gift/object
    c.glow(960, 600, 260, 0.35 * e, (255, 210, 130))
    c.rect((860, 620, 1060, 760), (20, 26, 44), width=3, round_=12)
    c.text((960, 690), "VALUE", 36, GOLD, alpha=e)
    c.text((960, 860), "you still have worth", 32, WHITE, kind="reg", alpha=e)


def m_doubt_certain(c, t, k):
    _lead(c, t, "DOUBT -> CERTAINTY")
    e = ease_in_out_sine(min(1.0, k * 1.5))
    c.text((W / 2, 560), "doubt", 30, GREY, kind="reg")
    c.text((W / 2, 720), "certainty", 30, GREEN, kind="reg")
    bar_x, bar_y, bar_w, bar_h = 500, 640, 920, 30
    c.rect((bar_x, bar_y, bar_x + bar_w, bar_y + bar_h), (20, 26, 40),
           round_=15)
    c.rect((bar_x, bar_y, bar_x + bar_w * e, bar_y + bar_h), RED, round_=15)


def m_battery(c, t, k):
    _lead(c, t, "TAKES REAL PSYCHOLOGICAL ENERGY")
    e = ease_in_out_cubic(min(1.0, k * 1.6))
    bx, by = 800, 560
    c.rect((bx, by, bx + 320, by + 160), (16, 22, 36), width=3, round_=12)
    c.rect((bx + 320, by + 50, bx + 360, by + 110), (40, 46, 60), round_=6)
    # drain
    fill = int(5 * (1 - e))
    for i in range(5):
        x0 = bx + 12 + i * 60
        col = ORANGE if i < fill else (10, 14, 24)
        c.rect((x0, by + 12, x0 + 50, by + 148), col, round_=6)
    c.text((W / 2, 820), "her energy: depleted", 30, WHITE, kind="reg",
           alpha=e)


def m_theory_wrong(c, t, k):
    _lead(c, t, "MOST OF THE TIME THE THEORY IS WRONG")
    e = ease_in_out_sine(min(1.0, k * 1.4))
    _card(c, 960, 560, 640, 200, min(1.0, k * 1.2), "THE THEORY", "greener grass")
    if e > 0.55:
        cx, cy = 960, 560
        c.line(cx - 130, cy - 60, cx + 130, cy + 60, RED, width=10)
        c.line(cx - 130, cy + 60, cx + 130, cy - 60, RED, width=10)
    c.text((W / 2, 860), "mostly, she discovers it was wrong", 30, WHITE,
           kind="reg", alpha=e)


def m_work_her(c, t, k):
    _lead(c, t, "YOU DO THE WORK FOR HER")
    e = ease_in_out_cubic(min(1.0, k * 1.3))
    # energy sparks transfer from man to woman
    x1, y1 = 600, 640
    x2, y2 = 1320, 640
    c.poly([(x1 - 24, y1), (x1 + 24, y1), (x1 + 30, y1 + 140),
            (x1, y1 + 122), (x1 - 30, y1 + 140)], (30, 36, 50), 3)
    c.poly([(x2 - 22, y2), (x2 + 22, y2), (x2 + 28, y2 + 140),
            (x2, y2 + 122), (x2 - 28, y2 + 140)], (150, 110, 90), 3)
    for i in range(12):
        p = ((t * 0.6 + i * 0.08) % 1.0)
        px = x1 + (x2 - x1) * p
        py = y1 - 30 - math.sin(p * math.pi) * 80
        c.glow(px, py, 7, 0.7 * (1 - abs(p - 0.5) * 2), AMBER)
    c.text((W / 2, 860), "confirmation: free, delivered by you", 30, WHITE,
           kind="reg", alpha=e)


def m_test_material(c, t, k):
    _lead(c, t, "SHE TESTS YOUR MATERIAL")
    e = ease_in_out_cubic(min(1.0, k * 1.3))
    # crash-test mannequin silhouette
    m = 120
    cx, cy = 960, 620
    c.poly([(cx - 40, cy), (cx + 40, cy), (cx + 52, cy + m),
            (cx, cy + m * 0.86), (cx - 52, cy + m)], (30, 36, 50), 4)
    c.glow(cx, cy, 130, 0.4 * e, ORANGE)
    c.text((cx, cy - m - 40), "what are you made of?", 30, WHITE, kind="reg",
           alpha=e)


def m_fire_house(c, t, k):
    _lead(c, t, "A SMALL FIRE IN THE HOUSE")
    e = ease_in_out_sine(min(1.0, k * 1.3))
    # house outline
    c.line(700, 600, 1220, 600, (60, 62, 80), width=8)
    c.line(700, 600, 700, 860, (60, 62, 80), width=8)
    c.line(1220, 600, 1220, 860, (60, 62, 80), width=8)
    c.line(700, 860, 1220, 860, (60, 62, 80), width=8)
    c.line(700, 600, 960, 480, (60, 62, 80), width=8)
    c.line(960, 480, 1220, 600, (60, 62, 80), width=8)
    # flame
    fp = pulse(t, 1.1, 0.85, 1.1)
    c.glow(960, 760, 80, 0.7 * fp, (255, 120, 40))
    c.glow(960, 760, 40, 0.7 * fp, (255, 230, 150))
    c.text((960, 920), "you point, she pours", 32, ORANGE, kind="reg",
           alpha=e)


def m_pour(c, t, k):
    _lead(c, t, "SHE POURS GASOLINE ON IT")
    e = ease_in_out_sine(min(1.0, k * 1.3))
    fp = pulse(t, 1.1, 0.85, 1.1)
    c.glow(520, 720, 200, 0.5 * fp, (255, 120, 40))
    c.glow(520, 720, 100, 0.5 * fp, (255, 180, 70))
    c.poly([(420, 860), (520, 720), (620, 860)], (10, 8, 8), 4)
    c.rect((1250, 560, 1420, 840), (40, 32, 26), round_=16)
    c.line(1335, 780, 560, 860, (230, 150, 70), width=12)
    c.line(1335, 780, 560, 860, (255, 210, 130), width=5)
    c.text((W / 2, 950), "gasoline: it only makes it worse", 32, ORANGE,
           kind="reg", alpha=e)


def m_stronger(c, t, k):
    _lead(c, t, "WOMEN STAY WITH THE STRONGER")
    e = ease_in_out_cubic(min(1.0, k * 1.3))
    # a tall strong figure vs a shrinking weak one
    # strong
    sx = 720
    sh = 210
    c.poly([(sx - 34, 640), (sx + 34, 640), (sx + 44, 640 + sh),
            (sx, 640 + sh * 0.86), (sx - 44, 640 + sh)], (70, 90, 120), 4)
    # weak (shrinking)
    wx = 1200
    wh = 210 * (1 - 0.4 * e)
    c.poly([(wx - 24, 640), (wx + 24, 640), (wx + 30, 640 + wh),
            (wx, 640 + wh * 0.86), (wx - 30, 640 + wh)], (40, 40, 52), 4)
    c.text((sx, 380), "stronger", 34, GREEN, kind="reg", alpha=e)
    c.text((wx, 380), "weak", 34, RED, kind="reg", alpha=e)
    c.text((W / 2, 180), "she stays with strength", 40, AMBER, tracking=12,
           alpha=_fade_in(t))


def m_paradox(c, t, k):
    _lead(c, t, "THE PARADOX")
    e = ease_in_out_cubic(min(1.0, k * 1.3))
    c.text((840, 620), "capable of walking away", 34, GREEN, kind="reg",
           alpha=e)
    c.text((1080, 620), "more likely she stays", 34, WHITE, kind="reg",
           alpha=e)
    # arrow
    c.line(960, 560, 960, 700, AMBER, width=8)
    c.poly([(940, 700 - 60), (980, 700 - 60), (960, 700)], AMBER)
    c.text((W / 2, 840), "let go -> she holds on", 36, AMBER, tracking=8,
           alpha=_fade_in(t - 0.4))


def m_palm(c, t, k):
    _lead(c, t, "YOU'RE IN THE PALM OF HER HAND")
    e = ease_in_out_cubic(min(1.0, k * 1.3))
    # open palm
    c.poly([(740, 760), (1180, 760), (1220, 660), (1120, 700),
            (1040, 760)], (120, 90, 70), 6)
    # tiny man in palm
    px = 960 + (1 - e) * 300
    c.poly([(px - 16, 700), (px + 16, 700), (px + 20, 700 + 90),
            (px, 700 + 78), (px - 20, 700 + 90)], (20, 22, 30), 3)
    c.text((W / 2, 900), "dependent = tiny in her eyes", 32, RED, kind="reg",
           alpha=e)


def m_fuse(c, t, k):
    _lead(c, t, "MEN FUSE THEIR LIFE TO THE RELATIONSHIP")
    e = ease_in_out_sine(min(1.0, k * 1.4))
    # two interlocking rings
    for cx, col in ((760, ORANGE), (1160, BLUE)):
        for i in range(48):
            a = i / 48 * 2 * math.pi
            x0 = cx + 160 * math.cos(a); y0 = 560 + 160 * math.sin(a)
            c.glow(x0, y0, 10, 0.5, col)
    # overlap fill = fused
    c.glow(960, 560, 80, 0.5 * e, AMBER)
    c.text((W / 2, 860), "no exit on his side", 34, WHITE, kind="reg",
           alpha=e)


def m_exit(c, t, k):
    _lead(c, t, "WOMEN KEEP THE EXIT OPEN")
    e = ease_in_out_cubic(min(1.0, k * 1.3))
    # open door
    c.rect((880, 520, 1040, 860), (16, 22, 36), width=4, round_=6)
    c.glow(960, 690, 140, 0.6 * e, AMBER)
    c.text((960, 900), "she always has an exit", 34, GREEN, kind="reg",
           alpha=e)


def m_asym(c, t, k):
    _lead(c, t, "THAT ASYMMETRY IS FATAL")
    e = ease_in_out_cubic(min(1.0, k * 1.4))
    c.text((780, 700), "you", 30, GREY, kind="reg", alpha=e)
    c.text((1140, 700), "her", 30, GREY, kind="reg", alpha=e)
    c.rect((660, 640, 900, 700), (30, 40, 60), round_=8)   # small
    c.rect((1020, 560, 1260, 700), (60, 90, 120), round_=8) # big
    c.text((W / 2, 860), "unequal exits = fatal", 34, RED, kind="reg",
           alpha=e)


def m_radar_silence(c, t, k):
    _lead(c, t, "RADIO SILENCE: YOU GO DARK")
    e = ease_in_out_sine(min(1.0, k * 1.4))
    # radar ring + dark sweep
    cx, cy = 960, 600
    for i in range(3):
        r = 120 + i * 110
        for j in range(64):
            a = j / 64 * 2 * math.pi
            x0 = cx + r * math.cos(a); y0 = cy + r * math.sin(a)
            c.glow(x0, y0, 4, 0.25, (40, 80, 120))
    # dark sweep erases
    sweep = (t * 0.5) % 1.0
    a0 = sweep * 2 * math.pi
    for i in range(40):
        a = a0 + i * 0.01
        r = 120 + i * 8
        x0 = cx + r * math.cos(a); y0 = cy + r * math.sin(a)
        c.glow(x0, y0, 14, 0.8, (6, 8, 14))
    c.text((W / 2, 860), "no info = no map of you", 34, WHITE, kind="reg",
           alpha=e)


def m_read(c, t, k):
    _lead(c, t, "LEAVE IT ON READ")
    e = ease_in_out_sine(min(1.0, k * 1.4))
    c.rect((700, 380, 1220, 760), (16, 20, 32), width=2, round_=28)
    c.glow(960, 570, 340, 0.25 * e, (60, 130, 200))
    c.rect((790, 430, 1130, 510), (30, 44, 64), round_=18)
    c.text((960, 470), "are you there?", 28, WHITE, kind="reg")
    c.rect((850, 560, 1130, 630), (34, 50, 72), round_=18)
    c.text((990, 595), "...", 30, WHITE, kind="reg")
    c.text((1030, 560), "read", 24, GREY, kind="reg", anchor="lt")
    c.text((960, 820), "you don't reply", 34, GREEN, kind="reg", alpha=e)


def m_recat(c, t, k):
    _lead(c, t, "SHE RECALCULATES")
    e = ease_in_out_cubic(min(1.0, k * 1.3))
    c.rect((800, 560, 1120, 700), (16, 22, 36), width=3, round_=10)
    for i, val in enumerate(["1", "2", "3", "4", "5", "6", "7", "8", "9"]):
        x0 = 830 + (i % 3) * 110
        y0 = 580 + (i // 3) * 48
        c.text((x0 + 30, y0 + 24), val, 34, GREY, kind="reg")
    c.text((960, 780), "still doesn't want you", 34, RED, kind="reg", alpha=e)


def m_pings(c, t, k):
    _lead(c, t, "SHE TESTS AGAIN: 2, 4, 6 MONTHS")
    e = ease_in_out_sine(min(1.0, k * 1.4))
    c.line(300, 640, 1620, 640, (40, 46, 62), width=4)
    marks = [0.2, 0.5, 0.8]
    for i, m in enumerate(marks):
        x = 300 + m * 1320
        if t > 0.4 + i * 0.5:
            a = min(1.0, (t - (0.4 + i * 0.5)) / 0.4)
            c.glow(x, 640, 18, 0.8 * a, AMBER)
            c.glow(x, 640, 7, 1.0 * a, (255, 255, 255))
            c.text((x, 690), f"{2*(i+1)}mo", 26, WHITE, kind="reg", alpha=a)
    c.text((W / 2, 860), "she keeps checking for a long time", 34, WHITE,
           kind="reg", alpha=e)


def m_object(c, t, k):
    _lead(c, t, "YOU BECOME AN OBJECT")
    e = ease_in_out_cubic(min(1.0, k * 1.5))
    # man silhouette fades, cube forms
    c.poly([(720, 640), (720 + 80, 640), (720 + 100, 640 + 200),
            (720, 640 + 200)], (30, 36, 50), 3)
    # cube
    s = 120 * e
    cx, cy = 1200, 640
    c.poly([(cx - s, cy - s * 0.6), (cx + s, cy - s * 0.6),
            (cx + s, cy + s * 0.6), (cx - s, cy + s * 0.6)],
           (70, 70, 80), 3)
    c.text((W / 2, 860), "used when needed, ignored otherwise", 34, RED,
           kind="reg", alpha=e)


def m_crit(c, t, k):
    _lead(c, t, "EVERY BREATH GETS A NEGATIVE REACTION")
    e = ease_in_out_sine(min(1.0, k * 1.4))
    for i in range(30):
        x = 200 + (i * 137) % 1520
        y = 360 + (i * 89) % 520
        a = (0.6 + 0.4 * math.sin(t * 2 + i)) * e
        c.line(x - 16, y - 16, x + 16, y + 16, RED, width=5)
        c.line(x - 16, y + 16, x + 16, y - 16, RED, width=5)
        c.glow(x, y, 10, 0.3 * a, RED)
    c.text((W / 2, 900), "constant criticism", 36, RED, kind="reg", alpha=e)


def m_trial_over(c, t, k):
    _lead(c, t, "THE FREE TRIAL PERIOD IS OVER")
    e = ease_in_out_cubic(min(1.0, k * 1.4))
    bar_x, bar_y, bar_w, bar_h = 690, 500, 540, 420
    c.rect((bar_x, bar_y, bar_x + bar_w, bar_y + bar_h), (18, 22, 34),
           round_=16)
    drained = ease_in_out_cubic(e)
    top = bar_y + bar_h * drained
    c.rect((bar_x, top, bar_x + bar_w, bar_y + bar_h), RED, round_=16)
    pct = int(round((1 - drained) * 100))
    c.text((W / 2, 600), f"{pct}%", 60, WHITE)
    c.text((W / 2, 700), "time left", 28, GREY, kind="reg", alpha=e)


def m_reclass(c, t, k):
    _lead(c, t, "SHE HAS RECLASSIFIED YOU")
    e = ease_out_back(min(1.0, k * 1.3))
    # label flip
    rot = 0.0
    w, h = 520, 120
    cx, cy = 960, 620
    pts = []
    cos, sin = math.cos(rot), math.sin(rot)
    for dx, dy in [(-w/2, -h/2), (w/2, -h/2), (w/2, h/2), (-w/2, h/2)]:
        pts.append((cx + dx*cos - dy*sin, cy + dx*sin + dy*cos))
    c.poly(pts, (30, 26, 34))
    c.text((cx, cy), "disposable", 44, RED, alpha=e)
    c.text((W / 2, 860), "mentally gone", 34, GREY, kind="reg", alpha=_fade_in(t - 0.4))


def m_closure(c, t, k):
    _lead(c, t, "CALM, RESPECTFUL CLOSURE")
    e = ease_in_out_cubic(min(1.0, k * 1.4))
    c.text((W / 2, 520), "\"Okay. If you don't want this anymore, goodbye.\"",
           40, WHITE, kind="reg", alpha=e)
    # two figures part cleanly
    x1 = 820 - e * 200
    x2 = 1100 + e * 200
    c.poly([(x1 - 22, 720), (x1 + 22, 720), (x1 + 28, 720 + 150),
            (x1, 720 + 130), (x1 - 28, 720 + 150)], (40, 46, 60), 3)
    c.poly([(x2 - 22, 720), (x2 + 22, 720), (x2 + 28, 720 + 150),
            (x2, 720 + 130), (x2 - 28, 720 + 150)], (120, 100, 90), 3)
    c.text((W / 2, 920), "separate amicably, then close the file", 32, GREEN,
           kind="reg", alpha=e)


def m_return(c, t, k):
    _lead(c, t, "EVERYONE COMES BACK")
    e = ease_in_out_sine(min(1.0, k * 1.5))
    c.line(300, 560, 1620, 560, (40, 46, 62), width=4)
    c.line(300, 700, 1620, 700, (40, 46, 62), width=4)
    c.text((300, 510), "men", 30, AMBER, kind="reg")
    c.text((300, 650), "women", 30, BLUE, kind="reg")
    # men short
    mx = 300 + min(1.0, e) * 400
    c.glow(mx, 560, 16, 0.8, AMBER)
    # women long (travels slowly)
    wx = 300 + min(1.0, e * 0.5) * 1100
    c.glow(wx, 700, 16, 0.8, BLUE)
    c.text((1620, 560), "months", 26, GREY, kind="reg")
    c.text((1620, 700), "years", 26, GREY, kind="reg")
    c.text((W / 2, 860), "the difference is the timer", 34, WHITE, kind="reg",
           alpha=e)


def m_notremorse(c, t, k):
    _lead(c, t, "ALMOST NEVER GENUINE REMORSE")
    e = ease_in_out_cubic(min(1.0, k * 1.4))
    _card(c, 760, 580, 480, 150, min(1.0, k * 1.2), "REMORSE", "rare")
    _card(c, 1160, 580, 480, 150, min(1.0, (k - 0.3) * 1.4),
          "TEST CONTROL", "common")
    c.text((W / 2, 860), "returns to check the buttons", 32, WHITE, kind="reg",
           alpha=e)


def m_extract(c, t, k):
    _lead(c, t, "SHE WANTS TO EXTRACT")
    e = ease_in_out_sine(min(1.0, k * 1.4))
    items = ["attention", "validation", "resources", "status"]
    for i, it in enumerate(items):
        x = 660 + i * 220
        if t > 0.3 + i * 0.4:
            a = min(1.0, (t - (0.3 + i * 0.4)) / 0.4)
            c.text((x, 560), it, 34, WHITE, kind="reg", alpha=a)
            c.line(x, 600, x, 700, ORANGE, width=6)
            c.poly([(x - 16, 700), (x + 16, 700), (x, 730)], ORANGE)
    c.text((W / 2, 860), "not you — what you provide", 34, ORANGE, kind="reg",
           alpha=e)


def m_netloss(c, t, k):
    _lead(c, t, "TAKING HER BACK IS A NET LOSS")
    e = ease_in_out_cubic(min(1.0, k * 1.4))
    c.text((W / 2, 600), "−", 200, RED, alpha=e)
    c.text((W / 2, 860), "you lose more than you gain", 34, RED, kind="reg",
           alpha=e)


def m_naivety(c, t, k):
    _lead(c, t, "LOVE REQUIRES NAIVETY")
    e = ease_in_out_sine(min(1.0, k * 1.4))
    # heart
    for i in range(120):
        a = i / 120 * 2 * math.pi
        hx = 960 + 130 * math.sin(a) ** 3
        hy = 600 - 110 * math.cos(a) + 40 * math.cos(2 * a)
        c.glow(hx, hy, 12, 0.5 * e, AMBER)
    c.glow(960, 600, 200, 0.3 * e, AMBER)
    c.text((W / 2, 850), "innocence is the cost of entry", 32, WHITE,
           kind="reg", alpha=e)


def m_lose(c, t, k):
    _lead(c, t, "AFTER ENOUGH CYCLES, MEN LOSE IT")
    e = ease_in_out_sine(min(1.0, k * 1.4))
    # heart dims with cycles
    for i in range(6):
        x = 600 + i * 140
        bright = 0.5 - i * 0.07
        c.glow(x, 600, 40, max(0.05, bright) * e, AMBER)
        c.text((x, 680), f"cycle {i+1}", 24, GREY, kind="reg", alpha=e)
    c.text((W / 2, 860), "naivety fades", 34, GREY, kind="reg", alpha=e)


def m_price(c, t, k):
    _lead(c, t, "EVERYTHING HAS A PRICE")
    e = ease_in_out_cubic(min(1.0, k * 1.3))
    # house with a price tag
    c.line(700, 600, 1220, 600, (60, 62, 80), width=8)
    c.line(700, 600, 700, 860, (60, 62, 80), width=8)
    c.line(1220, 600, 1220, 860, (60, 62, 80), width=8)
    c.line(700, 860, 1220, 860, (60, 62, 80), width=8)
    c.line(700, 600, 960, 480, (60, 62, 80), width=8)
    c.line(960, 480, 1220, 600, (60, 62, 80), width=8)
    # price tag swinging
    px = 1220
    swing = math.sin(t * 2) * 0.15
    c.line(px, 600, px, 560, GOLD, width=4)
    c.line(px, 560, px + 90, 560 + 30, GOLD, width=4)
    c.rect((px + 90, 560 + 30, px + 210, 560 + 110), GOLD, round_=8)
    c.text((px + 150, 560 + 70), "$", 40, (30, 24, 14), alpha=e)
    c.text((W / 2, 900), "a relationship has a price too", 32, WHITE,
           kind="reg", alpha=e)


def m_diminishing(c, t, k):
    _lead(c, t, "DIMINISHING RETURNS")
    e = ease_in_out_sine(min(1.0, k * 1.4))
    # a curve that plateaus then falls
    xs = [560 + i * 56 for i in range(15)]
    ys = []
    for i, x in enumerate(xs):
        y = 720 - 260 * (1 - math.exp(-i / 4.0)) * e
        ys.append(y)
    for i in range(len(xs) - 1):
        c.line(xs[i], ys[i], xs[i+1], ys[i+1], ORANGE, width=6)
    c.text((W / 2, 860), "output falls, investment keeps rising", 32, WHITE,
           kind="reg", alpha=e)


def m_scale_tips(c, t, k):
    _lead(c, t, "PRICE EXCEEDS VALUE")
    e = ease_in_out_cubic(min(1.0, k * 1.4))
    tilt = 0.3 * e
    pivot = (960, 440)
    arm = 420
    ca, sa = math.cos(tilt), math.sin(tilt)
    lx, ly = pivot[0] - arm * ca, pivot[1] + arm * sa
    rx, ry = pivot[0] + arm * ca, pivot[1] - arm * sa
    c.line(pivot[0], pivot[1], lx, ly, (60, 56, 50), width=10)
    c.line(pivot[0], pivot[1], rx, ry, (60, 56, 50), width=10)
    c.poly([(lx - 150, ly), (lx + 150, ly), (lx + 90, ly + 110),
            (lx - 90, ly + 110)], (30, 28, 24))
    c.glow(lx, ly + 40, 130, 0.4, GOLD)
    c.text((lx, ly + 62), "PRICE", 30, GOLD)
    c.poly([(rx - 150, ry), (rx + 150, ry), (rx + 90, ry + 110),
            (rx - 90, ry + 110)], (30, 28, 24))
    c.glow(rx, ry + 40, 130, 0.4, (120, 200, 210))
    c.text((rx, ry + 62), "VALUE", 30, (150, 220, 230))
    c.text((W / 2, 880), "stop paying", 40, ORANGE, tracking=12,
           alpha=_fade_in(t - 0.5))


def m_rare_change(c, t, k):
    _lead(c, t, "PEOPLE RARELY CHANGE")
    e = ease_in_out_sine(min(1.0, k * 1.4))
    # loop arrow
    cx, cy = 960, 600
    c.line(cx - 100, cy, cx + 100, cy, GREY, width=8)
    c.line(cx, cy, cx, cy - 60, GREY, width=8)
    c.poly([(cx - 24, cy - 60), (cx + 24, cy - 60), (cx, cy - 100)], GREY)
    c.text((W / 2, 860), "most stay roughly who they are", 34, WHITE, kind="reg",
           alpha=e)


def m_exception(c, t, k):
    _lead(c, t, "THE EXCEPTION, NOT THE RULE")
    e = ease_in_out_sine(min(1.0, k * 1.4))
    for i in range(20):
        x = 400 + (i * 121) % 1120
        y = 520 + (i * 71) % 240
        a = 0.5 * e
        c.glow(x, y, 10, a, GREY)
    # one green dot
    c.glow(1120, 620, 16, 0.9 * e, GREEN)
    c.glow(1120, 620, 6, 1.0 * e, (200, 255, 220))
    c.text((W / 2, 860), "genuine growth is rare", 34, GREEN, kind="reg",
           alpha=e)


def m_protect(c, t, k):
    _lead(c, t, "PROTECT WHAT YOU'RE BUILDING")
    e = ease_in_out_cubic(min(1.0, k * 1.4))
    # stack of blocks
    blocks = ["discipline", "income", "skills", "control"]
    for i, b in enumerate(blocks):
        y = 720 - i * 60
        if t > 0.3 + i * 0.4:
            a = min(1.0, (t - (0.3 + i * 0.4)) / 0.4)
            c.rect((760, y - 40, 1160, y), (40, 70, 100), round_=8)
            c.text((960, y - 20), b, 30, WHITE, kind="reg", alpha=a)
    # shield around it
    if e > 0.5:
        sh = min(1.0, (e - 0.5) / 0.4)
        c.glow(960, 640, 300, 0.3 * sh, (90, 200, 130))
        c.line(700, 640, 960, 480, GREEN, width=12)
        c.line(960, 480, 1220, 640, GREEN, width=12)
        c.line(700, 640, 1220, 640, GREEN, width=12)
    c.text((W / 2, 900), "don't let someone dismantle it", 32, RED, kind="reg",
           alpha=_fade_in(t - 0.6))


def m_checklist(c, t, k):
    _lead(c, t, "THE STRATEGIC FRAME")
    lines = ["invest in yourself first", "keep the exit open",
             "never beg / chase", "radio silence when needed",
             "calm closure", "don't mistake return for value"]
    for i, line in enumerate(lines):
        y = 300 + i * 84
        if t > 0.3 + i * 0.5:
            kk = min(1.0, (t - (0.3 + i * 0.5)) / 0.5)
            e = ease_out_back(kk)
            c.text((820, y), line, 36, WHITE, kind="reg",
                   tracking=int(12 * (1 - e)))
            if kk > 0.4:
                c.text((700, y), "✔", 36, GREEN, alpha=ease_in_out_sine(kk))
    if t > 3.6:
        c.text((W / 2, 880), "NEVER BEG", 100, AMBER, tracking=26,
               alpha=_fade_in(t - 3.6))


# --------------------------------------------------------------------------
# v3.1 — modern semantic motif library (gap-fillers + photo tiles)
# --------------------------------------------------------------------------
_CUR_SID = None


def _orb(c, cx, cy, r, k, col=AMBER, inner=(255, 240, 200)):
    """Modern glowing orb with a ring and hot core."""
    e = ease_out_back(k)
    c.glow(cx, cy, r * 2.0, 0.32 * e, col)
    for i in range(72):
        a = i / 72 * 2 * math.pi
        rr = r * e
        c.glow(cx + rr * math.cos(a), cy + rr * math.sin(a), 5,
               0.35 * e, col)
    c.glow(cx, cy, r * 0.42 * e, 0.55 * e, inner)


def _photo_tile(c, x, y, w, h, ccx=0.5, ccy=0.5, dim=0.80, radius=18,
                border=(16, 20, 34)):
    """Embed a framed photo crop of the section's AI backdrop (a 'pic')."""
    sid = _CUR_SID
    if not sid:
        return False
    bd = _backdrop(sid)
    if bd is None:
        return False
    arr, nw, nh = bd
    cw, ch = int(nw * 0.65), int(nh * 0.65)
    x0 = int((nw - cw) * ccx)
    y0 = int((nh - ch) * ccy)
    tile = arr[y0:y0 + ch, x0:x0 + cw]
    tile = np.asarray(Image.fromarray(tile.astype(np.uint8))
                      .resize((int(w), int(h)), Image.LANCZOS),
                      dtype=np.float32) * dim
    xs, xe = int(x - w / 2), int(x + w / 2)
    ys, ye = int(y - h / 2), int(y + h / 2)
    if 0 <= xs < xe <= W and 0 <= ys < ye <= H:
        c.img[ys:ye, xs:xe] = tile
    c.rect((xs, ys, xe, ye), border, width=3, round_=radius)
    c.rect((xs + 9, ys + 9, xe - 9, ye - 9), (20, 26, 42),
           width=1, round_=radius - 7)
    return True


def _glass_card(c, x, y, w, h, k, accent=TEAL, alpha=1.0):
    """Translucent glass panel: dark body, soft face, top highlight."""
    e = ease_out_back(k) * alpha
    yy = y + (1 - e) * 60
    c.glow(x, yy, max(w, h) * 0.5, 0.16 * e, accent)
    c.rect((x - w / 2, yy - h / 2, x + w / 2, yy + h / 2),
           (16, 22, 38), width=2, round_=22)
    c.rect((x - w / 2 + 5, yy - h / 2 + 5, x + w / 2, yy + h / 2),
           (30, 44, 70), width=0, round_=17)
    c.line(x - w / 2 + 20, yy - h / 2 + 16, x + w / 2 - 20,
           yy - h / 2 + 16, (255, 255, 255), width=2)
    return e


def m_reason3(c, t, k):
    _lead(c, t, "REASON THREE")
    _orb(c, 960, 560, 175, k, BLUE, (232, 246, 255))
    c.text((960, 560), "3", 150, WHITE, alpha=ease_out_back(k))
    c.text((W / 2, 860), "the strategic error", 32, GREY, kind="reg",
           alpha=ease_in_out_sine(min(1.0, k * 1.4)))


def m_strategic_error(c, t, k):
    _lead(c, t, "THE STRATEGIC ERROR")
    e = ease_out_back(k)
    cx, cy = 960, 580
    c.poly([(cx, cy - 200), (cx + 215, cy + 135), (cx - 215, cy + 135)],
           (70, 26, 22), blur=14)
    c.poly([(cx, cy - 168), (cx + 182, cy + 112), (cx - 182, cy + 112)],
           (150, 48, 38))
    if e > 0.4:
        a = min(1.0, (e - 0.4) / 0.4)
        c.text((cx, cy - 56), "!", 128, WHITE, alpha=a)
        c.glow(cx, cy + 78, 26, 0.7 * a, (255, 170, 90))
    c.text((cx, cy + 260), "fusing life to relationship", 34, RED,
           kind="reg", alpha=e)


def m_question(c, t, k):
    e = ease_in_out_sine(min(1.0, k * 1.3))
    _orb(c, 960, 550, 148, k, AMBER, (255, 240, 200))
    c.text((960, 535), "?", 172, (30, 22, 14), alpha=e)
    for i in range(10):
        a = t * 0.9 + i / 10 * 2 * math.pi
        c.glow(960 + 225 * math.cos(a), 550 + 225 * math.sin(a), 8,
               0.32 * e, WHITE)
    c.text((W / 2, 850), "she begins testing your material", 32, WHITE,
           kind="reg", alpha=e)


def m_close_file(c, t, k):
    _lead(c, t, "CLOSE THE FILE")
    e = ease_out_back(k)
    x, y, w, h = 960, 600, 640, 330
    c.rect((x - w / 2, y - h / 2, x + w / 2, y - h / 2 + 74),
           (44, 50, 82), round_=12)
    c.text((x, y - h / 2 + 37), "never_beg.txt", 30, WHITE, kind="reg")
    slide = ease_in_out_cubic(min(1.0, k * 1.4))
    top = y + h / 2 * (1 - slide)
    c.rect((x - w / 2, top, x + w / 2, y + h / 2), (18, 24, 40),
           width=3, round_=14)
    c.text((x, y + 120), "ARCHIVED", 30, GREY, kind="reg",
           alpha=min(1.0, slide))
    if slide > 0.7:
        a = min(1.0, (slide - 0.7) / 0.3)
        c.glow(x, y, 56, 0.6 * a, GREEN)
        c.text((x, y), "✔", 54, GREEN, alpha=a)
    c.text((W / 2, 950), "no looking back", 32, WHITE, kind="reg", alpha=e)


def m_deeper_lesson(c, t, k):
    _lead(c, t, "THE DEEPER LESSON")
    e = ease_in_out_sine(min(1.0, k * 1.4))
    for i, lbl in enumerate(["price", "value", "return"]):
        yy = 620 - i * 52 + (1 - e) * 90
        c.glow(960, yy, 320, 0.10 * (i + 1), GOLD)
        _glass_card(c, 960, yy, 720, 74, min(1.0, k * 1.2 + i * 0.15),
                    GOLD, alpha=0.95)
        c.text((960, yy), lbl, 34, GOLD if i == 0 else GREY, kind="reg")
    c.text((W / 2, 870), "what it costs you is hidden", 32, WHITE,
           kind="reg", alpha=_fade_in(t - 0.5))


def m_photo_moment(c, t, k, side="right"):
    """Fill a quiet beat with a large framed photo + short caption."""
    e = ease_in_out_cubic(min(1.0, k * 1.3))
    px = 1230 if side == "right" else 690
    _photo_tile(c, px, 560, 720, 470, ccx=0.5, ccy=0.4, dim=0.9)
    if e > 0.35:
        a = min(1.0, (e - 0.35) / 0.5)
        c.rect((px, 830, px + 400, 930), (14, 18, 30), width=2, round_=14)
        c.text((px + 200, 880), "the image of her walking out", 28,
               WHITE, kind="reg", alpha=a)
    return e


def m_photo_intro(c, t, k):
    """Opening: big photo, hero title in the center."""
    _photo_tile(c, 960, 540, 1500, 780, ccx=0.5, ccy=0.5, dim=0.78)
    c.rect((610, 410, 1310, 620), (8, 10, 18), width=0, round_=24)
    c.text((960, 470), "NEVER BEG", 96, AMBER, tracking=24,
           alpha=_fade_in(t, 0.7))
    c.text((960, 560), "a woman walks out", 34, WHITE, kind="reg",
           alpha=_fade_in(t - 0.5))


# --------------------------------------------------------------------------
# Section definitions: (mood, [(sentence, motif_fn), ...])
# --------------------------------------------------------------------------
SECTIONS = {
    "s01": ("amber", [
        ("Never beg a woman who's walking out of the relationship.", m_photo_intro),
        ("There are three core reasons, and I'm going to break them down clearly.", m_count3),
    ]),
    "s02": ("cool", [
        ("Reason one.", m_photo_moment),
        ("She's never one hundred percent sure.", m_gauge_near),
        ("The girl who is leaving is almost never completely certain.", m_gauge_near),
        ("She has a theory.", m_gauge_near),
        ("She suspects the grass is greener.", m_grass),
        ("She believes her market value is higher than what she's getting from you.", m_market),
        ("She's running an experiment in her head: maybe I can do better.", m_experiment),
        ("When you chase her, you hand her free confirmation of that theory.", m_chase_confirm),
        ("You prove that what she's walking away from still has value.", m_value_spot),
        ("Her doubt turns into certainty.", m_doubt_certain),
        ("She tells herself: look, he's running after me even after everything.", m_chase_confirm),
        ("I really was that valuable.", m_value_spot),
        ("I can definitely find someone better.", m_market),
        ("If you don't chase, you force her to test the theory herself.", m_experiment),
        ("That takes real psychological energy.", m_battery),
        ("Most of the time she discovers the theory was wrong.", m_theory_wrong),
        ("But the moment you chase, you do the work for her.", m_work_her),
    ]),
    "s03": ("red", [
        ("Reason two.", m_photo_moment),
        ("She is testing your material.", m_test_material),
        ("When you say, let's fix this, you're pointing at a small fire in the house.", m_fire_house),
        ("What does she often do?", m_question),
        ("She brings a gallon of gasoline and pours it on.", m_pour),
        ("Why? Because she needs to see what you're made of.", m_test_material),
        ("She needs to know whether you have the psychological strength to let her go.", m_stronger),
        ("This is the paradox.", m_paradox),
        ("The moment you are capable of walking away, she is more likely to stay.", m_paradox),
        ("The moment she senses you cannot walk away, she has you in the palm of her hand.", m_palm),
        ("Women stay with men they perceive as stronger than themselves.", m_stronger),
        ("When all your weakness comes out, you fall completely out of her eyes.", m_palm),
    ]),
    "s04": ("blue", [
        ("Reason three.", m_reason3),
        ("The strategic error most men make.", m_strategic_error),
        ("The root mistake is that men fuse their entire life to the relationship.", m_fuse),
        ("After two or three years, many men feel they literally cannot leave.", m_fuse),
        ("Women almost always keep the exit option open.", m_exit),
        ("That asymmetry is fatal.", m_asym),
        ("When she finally leaves, she already has a mental model of how you'll behave.", m_radar_silence),
        ("She expects you to be hurt, to struggle, to take time to replace her.", m_object),
        ("If you start posting party photos, she sees through it.", m_radar_silence),
        ("It's too fast. It looks performative. She knows it's not real.", m_read),
        ("The correct move is radio silence.", m_radar_silence),
        ("No posts. No information. In military terms, you go dark.", m_radar_silence),
        ("When she has no information, she will eventually reach out to gather intel.", m_read),
        ("The wrong play is to answer and start talking.", m_read),
        ("She recalculates and decides she still doesn't want you.", m_recat),
        ("The correct play is to leave the message on read.", m_read),
        ("Do not block. Just don't reply.", m_read),
        ("She will test again in two months, four months, six months.", m_pings),
        ("She will keep checking for a long time.", m_pings),
        ("In reality she is not thinking of you as a full person anymore.", m_object),
        ("You have become an object she uses and otherwise ignores.", m_object),
    ]),
    "s05": ("red", [
        ("When the constant criticism starts.", m_crit),
        ("Every breath you take receives a negative reaction.", m_crit),
        ("At that stage the free trial period is over.", m_trial_over),
        ("She has already mentally reclassified you.", m_reclass),
        ("The relationship is finished.", m_reclass),
        ("Trying to fix it by promising you'll change is the losing move.", m_closure),
        ("The clean move is calm, respectful closure.", m_closure),
        ("If you don't want this anymore, goodbye. We can separate amicably.", m_closure),
        ("Then you close the file and do not look back.", m_close_file),
    ]),
    "s06": ("cool", [
        ("Everyone comes back, on different timelines.", m_return),
        ("Men and women both return. The difference is the timer.", m_return),
        ("Men often return in months.", m_photo_moment),
        ("Women often return much later, sometimes years, and only when they are confident.", m_return),
        ("They return to check whether the old buttons still work.", m_notremorse),
        ("When she comes back it is almost never genuine remorse.", m_notremorse),
        ("Either she couldn't find better, or she wants to test control again.", m_notremorse),
        ("Or she wants to extract something: attention, validation, resources, status.", m_extract),
        ("Taking her back after she discarded you is usually a net loss.", m_netloss),
        ("She left when she believed better options existed. That hasn't changed.", m_notremorse),
    ]),
    "s07": ("gold", [
        ("The deeper lesson.", m_deeper_lesson),
        ("Love requires a certain amount of naivety.", m_naivety),
        ("After enough cycles, most men lose that naivety.", m_lose),
        ("They stop pouring resources into people who treat them as disposable.", m_netloss),
        ("They ask: what is the actual value of this relationship relative to the cost?", m_scale_tips),
        ("Everything has a price.", m_price),
        ("A house in a neighborhood has a market price.", m_price),
        ("A relationship that requires over-investment while delivering diminishing returns also has a price.", m_diminishing),
        ("When the price exceeds the value, the rational move is to stop paying.", m_scale_tips),
        ("People rarely change in the ways we hope.", m_rare_change),
        ("Most people, male or female, stay roughly who they are.", m_rare_change),
        ("The few who genuinely improve are the exception, not the rule.", m_exception),
        ("If you are already building yourself, protect that work.", m_protect),
        ("Do not let someone with unresolved issues walk in and dismantle it.", m_protect),
    ]),
    "s08": ("amber", [
        ("Invest in yourself first.", m_checklist),
        ("Keep the exit option open on your side.", m_checklist),
        ("Never beg. Never chase. Never over-explain.", m_checklist),
        ("Radio silence when necessary.", m_checklist),
        ("Calm closure when the relationship is clearly over.", m_checklist),
        ("And never confuse her later return with genuine value.", m_checklist),
        ("That is the strategic frame.", m_checklist),
    ]),
}

_DURS = {"s01": 6.5, "s02": 47.1, "s03": 47.4, "s04": 90.5,
         "s05": 29.0, "s06": 44.8, "s07": 60.9, "s08": 18.7}


def render_scene(sid, t):
    global _CUR_SID
    _CUR_SID = sid
    c = Canvas()
    mood, items = SECTIONS[sid]
    used_bg = _draw_backdrop(c, sid, t)
    _atmosphere(c, t, mood, base=not used_bg)
    durs = _DURS[sid]
    phrases = [p for p, _ in items]
    beats = _schedule(phrases, durs)
    # find active beat
    idx = 0
    for i, (b0, b1) in enumerate(beats):
        if t >= b0:
            idx = i
    b0, b1 = beats[idx]
    bt = t - b0
    bd = max(0.1, b1 - b0)
    k = min(1.0, bt / bd)

    _, fn = items[idx]
    if fn is not None and idx > 0:
        fn(c, bt, k)
    elif idx == 0 and fn is not None:
        # first beat: show the section title as a hero
        fn(c, bt, k)
    else:
        # fallback for None-typed beats: show the phrase as a caption
        a = min(1.0, bt / 0.6)
        c.text((W / 2, 560), phrases[idx][:70], 40, WHITE, kind="reg",
               alpha=a)

    return c.finish()
