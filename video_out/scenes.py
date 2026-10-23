#!/usr/bin/env python3
"""Animated scene renderers for the "Never Beg" voiceover.

Each function paints one 1920x1080 frame for section N at time `t` (seconds
from the start of that section) onto a motion_engine.Canvas. The scenes are
real motion graphics: parallax layers, animated silhouettes, type that
kern/tracks in, glow pulses, travel paths, counters, etc. All programmatic —
no AI, no asset files.

Semantic beats per section (sourced from the narration scripts):
  s01 intro   — headings kern/track in, "walking out" door pulse, 3 reasons count up.
  s02 reason1 — "Theory / Greener grass / Confirmation / Don't chase" cards fly in,
                 a "grass vs value bar" fills, chasing figure gets red "confirmed" stamp.
  s03 reason2 — "small fire" vs "gasoline pour" visual; stability dial paradox.
  s04 reason3 — "radio silence" dot blinks, message bubble stays "Read", a radar/spotlight sweep.
  s05 criticism — "free trial over" counter runs out, a red slash across "promises".
  s06 comeback — timeline with dots (men return months, women years) animating.
  s07 lesson  — value-vs-price balance scale tips with a lever animation; "protect your build".
  s08 close   — checklist of the strategy lines + exit door opens with light.
"""
from motion_engine import (Canvas, KF, W, H, FPS,
                           ease_out_back, ease_in_out_cubic, ease_in_out_sine,
                           ease_out_cubic, ease_out_bounce, pulse)
import math

TEAL = (40, 120, 140)
ORANGE = (230, 150, 70)
AMBER = (255, 190, 90)
GREEN = (90, 200, 130)
RED = (220, 70, 60)
WHITE = (235, 238, 244)
GREY = (120, 128, 142)
DARK = (8, 10, 16)
GOLD = (220, 190, 130)


def _lead_text(c, t, line, y, appear_t, size=72, color=WHITE, tracking=40):
    """Kern/track-in a headline line (starts wide, tightens)."""
    if t < appear_t:
        return
    k = min(1.0, (t - appear_t) / 0.9)
    e = ease_out_back(k)
    tr = int(round(tracking * (1 - e) + 4))
    alpha = min(1.0, k * 1.6)
    c.text((W / 2, y), line, size, color, kind="bold",
           tracking=tr, alpha=alpha)


def _sub(c, t, line, y, appear_t, size=34, color=GREY):
    if t < appear_t:
        return
    k = min(1.0, (t - appear_t) / 0.7)
    c.text((W / 2, y), line, size, color, kind="reg", alpha=ease_in_out_sine(k))


# ------------------------------------------------------------------ s01 ----
def s01(c, t):
    c.base_gradient([(0.0, (10, 14, 26)), (0.55, (20, 30, 50)),
                     (1.0, (6, 9, 18))])
    # a doorway glow on the right that the "woman" walks toward
    gx = 1510 + math.sin(t * 0.6) * 8
    c.glow(gx, 560, 420, 0.55, (255, 170, 80))
    c.glow(gx - 40, 540, 180, 0.5, (255, 210, 130))
    # two hallway silhouettes flanking the light
    c.poly([(0, 0), (760, 0), (760, 270), (480, 500), (0, 500)], (6, 8, 14), 6)
    c.poly([(1920, 0), (1250, 0), (1250, 300), (1480, 520), (1920, 520)],
           (6, 8, 14), 6)
    # walking figure drifts right toward the door
    fx = 560 + ease_in_out_cubic(min(1.0, (t - 1.0) / 5.0)) * 420
    fs = 1.0 + 0.05 * math.sin(t * 1.2)
    fh = 150 * fs
    c.poly([(fx - 26, 640), (fx + 26, 640), (fx + 34, 640 + fh),
            (fx, 640 + fh * 0.88), (fx - 34, 640 + fh)], (9, 12, 20), 4)
    _lead_text(c, t, "NEVER BEG", 300, 0.3, 110, AMBER, 60)
    _sub(c, t, "the frame that changes the game", 388, 1.4)
    # three reasons count up
    reasons = ["no chasing", "no over-explaining", "exit open"]
    for i, r in enumerate(reasons):
        y = 620 + i * 96
        if t > 2.2 + i * 0.7:
            k = min(1.0, (t - (2.2 + i * 0.7)) / 0.8)
            e = ease_out_back(k)
            c.text((W / 2, y), r, 44, WHITE, tracking=int(10 * (1 - e)))
            c.rect((W / 2 - 90, y - 28, W / 2 + 90, y + 28),
                   (12, 16, 28), width=2, round_=14)
    # bottom intro key
    if t > 4.5:
        k = min(1.0, (t - 4.5) / 0.8)
        c.text((W / 2, 930), "three reasons, one decision", 30,
               ORANGE, alpha=k)


# ------------------------------------------------------------------ s02 ----
def s02(c, t):
    c.base_gradient([(0.0, (12, 18, 30)), (0.6, (22, 34, 52)),
                     (1.0, (8, 12, 22))])
    _lead_text(c, t, "REASON 1", 150, 0.2, 96, AMBER, 60)
    _sub(c, t, "she's never 100% sure", 230, 0.9)
    # four metaphor cards fly in on a staggered grid
    cards = [("THEORY", "grass might be greener"),
             ("TEST", "she runs an experiment"),
             ("CONFIRM", "you chase -> she knows"),
             ("RESULT", "certainty for her")]
    cols, rows = 2, 2
    for i, (title, sub) in enumerate(cards):
        cx0 = W / 2 + (i % cols - 0.5) * 700
        cy0 = 460 + (i // rows) * 220
        appear = 1.6 + i * 0.45
        if t > appear:
            k = min(1.0, (t - appear) / 0.7)
            e = ease_out_back(k)
            yy = cy0 + (1 - e) * 90
            c.rect((cx0 - 320, yy - 70, cx0 + 320, yy + 70),
                   (14, 20, 34), width=2, round_=18)
            c.text((cx0, yy - 18), title, 42, AMBER, alpha=e)
            c.text((cx0, yy + 26), sub, 26, GREY, kind="reg", alpha=e)
    # theory/confirmation bar: "certainty" fills as you chase
    if t > 5.6:
        k = min(1.0, (t - 5.6) / 3.2)
        bar_x, bar_y, bar_w, bar_h = 460, 880, 1000, 24
        c.text((450, bar_y + 12), "certainty", 26, GREY, kind="reg",
               anchor="rm")
        c.rect((bar_x, bar_y, bar_x + bar_w, bar_y + bar_h),
               (20, 26, 40), round_=12)
        fill = bar_w * ease_in_out_sine(k)
        c.rect((bar_x, bar_y, bar_x + fill, bar_y + bar_h), RED, round_=12)
        c.text((bar_x + bar_w + 60, bar_y + 12), f"{int(k*100)}%",
               28, RED, anchor="lm")


# ------------------------------------------------------------------ s03 ----
def s03(c, t):
    c.base_gradient([(0.0, (30, 10, 8)), (0.5, (18, 8, 8)),
                     (1.0, (6, 6, 10))])
    _lead_text(c, t, "REASON 2", 140, 0.2, 96, AMBER, 60)
    _sub(c, t, "she tests your material", 220, 0.9)
    # small fire (left) burning low, pulsing
    fire_p = pulse(t, 1.1, 0.85, 1.1)
    c.glow(520, 720, 220, 0.5 * fire_p, (255, 120, 40))
    c.glow(520, 720, 120, 0.5 * fire_p, (255, 180, 70))
    c.glow(520, 720, 60, 0.7 * fire_p, (255, 230, 150))
    c.poly([(420, 860), (520, 720), (620, 860)], (10, 8, 8), 4)  # fire pit
    # gasoline jug (right) tilts and "pours" a stream onto the fire
    jug_swing = ease_in_out_sine(min(1.0, max(0.0, (t - 3.2) / 2.2)))
    angle = -0.5 + jug_swing * 1.5
    c.rect((1250, 560, 1420, 840), (40, 32, 26), round_=16)
    # pour line: a bright stream that reaches the fire
    if t > 3.2:
        k = min(1.0, (t - 3.2) / 1.4)
        # stream from jug mouth to fire
        x0, y0, x1, y1 = 1335, 780, 560, 860
        tip = k
        c.line(x0, y0, x0 + (x1 - x0) * tip, y0 + (y1 - y0) * tip,
               (230, 150, 70), width=10)
        c.line(x0, y0, x0 + (x1 - x0) * tip, y0 + (y1 - y0) * tip,
               (255, 210, 130), width=4)
    # "paradox" callout
    if t > 6.0:
        k = min(1.0, (t - 6.0) / 0.8)
        c.text((W / 2, 950), "pouring gasoline only proves you can't let go"
               + "  →  she loses respect", 32, ORANGE, alpha=k)


# ------------------------------------------------------------------ s04 ----
def s04(c, t):
    c.base_gradient([(0.0, (8, 10, 16)), (1.0, (20, 24, 36))])
    _lead_text(c, t, "REASON 3", 140, 0.2, 96, AMBER, 60)
    _sub(c, t, "the strategic error", 220, 0.9)
    # a phone UI: sent bubble, a "read" grey bubble, then a blip that goes dark
    c.rect((700, 320, 1220, 820), (16, 20, 32), width=2, round_=28)
    # screen glow
    c.glow(960, 570, 360, 0.28, (60, 130, 200))
    # message bubbles
    if t > 2.0:
        c.rect((790, 380, 1130, 460), (30, 44, 64), round_=18)
        c.text((960, 420), "are you there?", 28, WHITE, kind="reg")
    if t > 3.4:
        c.rect((850, 500, 1130, 570), (34, 50, 72), round_=18)
        c.text((990, 535), "...", 30, WHITE, kind="reg")
        c.text((1030, 500), "read", 22, GREY, kind="reg", anchor="lt")
    # radio-silence blip: a green "online" dot that goes grey -> gone
    if t > 5.0:
        phase = min(1.0, (t - 5.0) / 2.4)
        if phase < 0.6:
            a = 1.0 - phase / 0.6
            c.glow(1110, 760, 20, 0.9 * a, GREEN)
            c.glow(1110, 760, 8, 1.0 * a, (150, 255, 170))
        # then a fading "go dark" label
        if phase > 0.6:
            a = min(1.0, (phase - 0.6) / 0.4)
            c.text((960, 760), "go dark", 34, RED, alpha=a)
    # spotlight sweep sweeping the room (the "no info for her" idea)
    sweep = (t * 0.25) % 1.0
    sx = 400 + sweep * 1120
    c.glow(sx, 900, 260, 0.16, (180, 200, 230))


# ------------------------------------------------------------------ s05 ----
def s05(c, t):
    c.base_gradient([(0.0, (6, 6, 10)), (0.5, (14, 14, 22)),
                     (1.0, (4, 5, 8))])
    _lead_text(c, t, "WHEN CRITICISM BEGINS", 150, 0.2, 80, AMBER, 40)
    _sub(c, t, "the free trial is over", 230, 1.0)
    # a "trial bar" draining down like a timer
    bar_x, bar_y, bar_w, bar_h = 690, 500, 540, 420
    c.text((W / 2, 440), "free trial", 30, GREY, kind="reg")
    c.rect((bar_x, bar_y, bar_x + bar_w, bar_y + bar_h),
           (18, 22, 34), round_=16)
    drained = ease_in_out_cubic(min(1.0, (t - 2.0) / 6.0))
    top = bar_y + bar_h * drained
    c.rect((bar_x, top, bar_x + bar_w, bar_y + bar_h), RED, round_=16)
    # a percentage counter dropping
    pct = int(round((1 - drained) * 100))
    c.text((W / 2, 560), f"{pct}%", 56, WHITE, kind="bold")
    # a red slash across "promises to change" once it's over
    if t > 8.0:
        k = min(1.0, (t - 8.0) / 0.5)
        c.text((W / 2, 760), "promises to change", 40, WHITE,
               kind="reg", alpha=k)
        if k > 0.3:
            c.line(720, 740, 1200, 800, RED, width=8)
    # calm closure line
    if t > 10.0:
        k = min(1.0, (t - 10.0) / 0.7)
        c.text((W / 2, 940), "calm, respectful closure", 38, GREEN, alpha=k)


# ------------------------------------------------------------------ s06 ----
def s06(c, t):
    c.base_gradient([(0.0, (16, 14, 22)), (0.5, (30, 28, 40)),
                     (1.0, (8, 8, 14))])
    _lead_text(c, t, "EVERYONE COMES BACK", 150, 0.2, 84, AMBER, 40)
    _sub(c, t, "on different timelines", 230, 1.0)
    # a horizontal timeline with two lane tracks: men (short) vs women (long)
    y_men, y_women = 520, 700
    c.line(300, y_men, 1620, y_men, (40, 46, 62), width=4)
    c.line(300, y_women, 1620, y_women, (40, 46, 62), width=4)
    c.text((300, y_men - 50), "men", 30, WHITE, kind="reg", anchor="lm")
    c.text((300, y_women - 50), "women", 30, WHITE, kind="reg", anchor="lm")
    # markers travel along: men return early, women return late
    men_speed, women_speed = 0.35, 0.14
    for i in range(2):
        m = min(1.0, (t - 2.0) * men_speed + i * 0.3)
        if 0 < m < 1:
            c.poly([(300 + m * 1320 - 14, y_men - 16),
                    (300 + m * 1320 + 14, y_men - 16),
                    (300 + m * 1320, y_men + 16)], (255, 190, 90))
    for i in range(2):
        m = min(1.0, (t - 3.0) * women_speed + i * 0.2)
        if 0 < m < 1:
            c.poly([(300 + m * 1320 - 14, y_women - 16),
                    (300 + m * 1320 + 14, y_women - 16),
                    (300 + m * 1320, y_women + 16)], (120, 190, 220))
    # end-of-timeline captions
    c.text((1620, y_men), "months", 28, GREY, kind="reg", anchor="lm")
    c.text((1620, y_women), "years", 28, GREY, kind="reg", anchor="lm")
    # "almost never remorse" reveal
    if t > 6.0:
        k = min(1.0, (t - 6.0) / 0.7)
        c.text((W / 2, 900), "almost never remorse  →  control / extraction",
               32, RED, alpha=k)


# ------------------------------------------------------------------ s07 ----
def s07(c, t):
    c.base_gradient([(0.0, (22, 20, 16)), (0.6, (14, 14, 14)),
                     (1.0, (8, 8, 10))])
    _lead_text(c, t, "PRICE vs VALUE", 150, 0.2, 96, AMBER, 60)
    _sub(c, t, "stop paying when price exceeds value", 230, 1.0)
    # a balance scale: left pan "price" (gold), right pan "value" (house)
    # the beam tilts as cost outweighs value
    tilt = KF([(0.0, 0.0), (5.0, 0.0), (8.0, 0.28)], ease_in_out_cubic).at(t)
    pivot = (960, 400)
    arm = 420
    ang = tilt
    ca, sa = math.cos(ang), math.sin(ang)
    lx, ly = pivot[0] - arm * ca, pivot[1] + arm * sa
    rx, ry = pivot[0] + arm * ca, pivot[1] - arm * sa
    c.line(pivot[0], pivot[1], lx, ly, (60, 56, 50), width=10)
    c.line(pivot[0], pivot[1], rx, ry, (60, 56, 50), width=10)
    # left pan (price) sits lower -> fuller, heavy gold
    c.poly([(lx - 150, ly), (lx + 150, ly), (lx + 90, ly + 110),
            (lx - 90, ly + 110)], (30, 28, 24))
    c.glow(lx, ly + 40, 130, 0.4, (230, 190, 120))
    c.text((lx, ly + 55), "PRICE", 30, GOLD, kind="bold")
    # right pan (value)
    c.poly([(rx - 150, ry), (rx + 150, ry), (rx + 90, ry + 110),
            (rx - 90, ry + 110)], (30, 28, 24))
    c.glow(rx, ry + 40, 130, 0.4, (120, 200, 210))
    c.text((rx, ry + 55), "VALUE", 30, (150, 220, 230), kind="bold")
    # verdict banner appears once tilted
    if t > 7.5:
        k = min(1.0, (t - 7.5) / 0.6)
        c.text((W / 2, 880), "the rational move: stop paying", 38, ORANGE,
               alpha=1.0 if k > 0.3 else k)
    # protect-your-build footer
    if t > 10.0:
        k = min(1.0, (t - 10.0) / 0.7)
        c.text((W / 2, 960), "protect the work you've built", 30, GREEN, alpha=k)


# ------------------------------------------------------------------ s08 ----
def s08(c, t):
    c.base_gradient([(0.0, (6, 8, 16)), (0.6, (16, 22, 36)),
                     (1.0, (34, 44, 62))])
    # a bright open door at the end of a corridor
    door_open = ease_out_cubic(min(1.0, (t - 1.2) / 2.2))
    c.glow(1500, 560, 420, 0.5, (255, 200, 110))
    c.glow(1500, 560, 200, 0.5, (255, 235, 180))
    # corridor walls converge (parallax as we "walk")
    shift = (t * 0.6) % 60
    c.poly([(0, 0), (760 - shift, 220), (760 - shift, 700), (0, 1080)],
           (5, 6, 12), 10)
    c.poly([(1920, 0), (1180 + shift, 220), (1180 + shift, 700), (1920, 1080)],
           (5, 6, 12), 10)
    # a checklist of the strategy lines grows
    check = ["invest in yourself first", "keep the exit open",
             "never beg / chase", "radio silence when needed",
             "calm closure", "don't mistake return for value"]
    for i, line in enumerate(check):
        y = 300 + i * 78
        if t > 3.0 + i * 0.9:
            k = min(1.0, (t - (3.0 + i * 0.9)) / 0.6)
            e = ease_out_back(k)
            c.text((820, y), line, 36, WHITE, kind="reg",
                   tracking=int(14 * (1 - e)))
            if k > 0.4:
                c.text((700, y), "✔", 36, GREEN, kind="bold", alpha=ease_in_out_sine(k))
    # final title
    if t > 9.5:
        c.text((W / 2, 900), "NEVER BEG", 110, AMBER, kind="bold",
               tracking=30, alpha=ease_in_out_sine(min(1.0, (t - 9.5) / 0.8)))


SCENES = {
    "s01": s01, "s02": s02, "s03": s03, "s04": s04,
    "s05": s05, "s06": s06, "s07": s07, "s08": s08,
}


def render_scene(sid, t):
    """Render one frame -> uint8 (H,W,3)."""
    c = Canvas()
    SCENES[sid](c, t)
    return c.finish()
