#!/usr/bin/env python3
"""Motion engine for the "Never Beg" voiceover video.

A tiny procedural motion-graphics renderer. No AI, no ffmpeg filters for the
visuals themselves — just numpy + Pillow. Each scene is a function that paints
onto a Canvas given time `t` (seconds into the section). The engine provides:

  * Easing curves (out-back, in-out-cubic, out-bounce, in-out-sine).
  * A small keyframe interpolator (KF) for timed property values.
  * The Canvas: additive glows, gradient bases, silhouettes, spaced text,
    vignette + film-grain finish. Vector drawing accumulates on one persistent
    RGBA layer and is composited over the gradient base once per frame.

Render at 1920x1080 / 30 fps, pipe raw RGB straight into ffmpeg.

Perf notes: the coordinate grid, per-gradient base, and grain/vignette masks
are cached once; glows only compute inside their bounding box.
"""
import math
import numpy as np
from PIL import Image, ImageDraw, ImageFilter, ImageFont

W, H = 1920, 1080
FPS = 30

# ------------------------------------------------------------- cached grids --
_YY = _XX = None
def _grids():
    global _YY, _XX
    if _YY is None or _YY.shape[0] != H:
        _YY, _XX = np.mgrid[0:H, 0:W].astype(np.float32)
    return _YY, _XX

# ---------------------------------------------------------------- fonts -----
FONT_BOLD = "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf"
FONT_REG = "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"
FONT_MONO = "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf"

_font_cache = {}
def font(kind, size):
    key = (kind, size)
    if key not in _font_cache:
        path = {"bold": FONT_BOLD, "reg": FONT_REG,
                "mono": FONT_MONO}[kind]
        _font_cache[key] = ImageFont.truetype(path, size)
    return _font_cache[key]

# ---------------------------------------------------------------- easing ----
def clamp01(x):
    return max(0.0, min(1.0, x))

def ease_out_back(t):
    t = clamp01(t)
    c1, c3 = 1.70158, 2.70158
    return 1 + c3 * (t - 1) ** 3 + c1 * (t - 1) ** 2

def ease_in_out_cubic(t):
    t = clamp01(t)
    return 4 * t ** 3 if t < 0.5 else 1 - ((-2 * t + 2) ** 3) / 2

def ease_out_cubic(t):
    t = clamp01(t)
    return 1 - (1 - t) ** 3

def ease_in_out_sine(t):
    t = clamp01(t)
    return -(math.cos(math.pi * t) - 1) / 2

def ease_out_bounce(t):
    t = clamp01(t)
    n1, n2, n3 = 7.5625, 2.75, 2.625
    if t < 1 / n2:
        return n1 * t * t
    if t < 2 / n2:
        return n1 * (t - 1.5 / n2) ** 2 + 0.75
    if t < 2.5 / n2:
        return n1 * (t - 2.25 / n2) ** 2 + 0.9375
    return n1 * (t - 2.625 / n2) ** 2 + 0.984375

def pulse(t, period=1.0, lo=0.85, hi=1.0):
    """Smooth loop for pulsing scale/alpha in [lo, hi]."""
    x = (t % period) / period
    return lo + (hi - lo) * (0.5 - 0.5 * math.cos(2 * math.pi * x))

# ---------------------------------------------------------------- keyframe --
class KF:
    """Piecewise linear keyframe interpolator with per-stop easing.

    points: list of (t, value). value may be float or (r,g,b) tuple.
    ease: easing fn applied per segment (None => linear).
    """

    def __init__(self, points, ease=None):
        self.points = sorted(points, key=lambda p: p[0])
        self.ease = ease

    def at(self, t):
        pts = self.points
        if t <= pts[0][0]:
            return pts[0][1]
        if t >= pts[-1][0]:
            return pts[-1][1]
        for i in range(len(pts) - 1):
            t0, v0 = pts[i]
            t1, v1 = pts[i + 1]
            if t0 <= t <= t1:
                if t1 == t0:
                    return v1
                f = (t - t0) / (t1 - t0)
                if self.ease:
                    f = self.ease(f)
                if isinstance(v0, (tuple, list)):
                    return tuple(a + (b - a) * f for a, b in zip(v0, v1))
                return v0 + (v1 - v0) * f
        return pts[-1][1]

# ---------------------------------------------------------------- gradients --
_grad_cache = {}
def _cached_gradient(stops):
    key = tuple(stops)
    if key in _grad_cache:
        return _grad_cache[key]
    yy = (np.arange(H) / (H - 1))[:, None]
    base = np.zeros((H, W, 3), dtype=np.float32)
    for i in range(len(stops) - 1):
        p0, c0 = stops[i]
        p1, c1 = stops[i + 1]
        t = np.clip((yy - p0) / (p1 - p0), 0, 1)
        seg = (yy >= p0) & (yy <= p1)
        col = (np.array(c0, dtype=float) * (1 - t) +
               np.array(c1, dtype=float) * t)          # (H,3)
        col = col[:, None, :]                           # (H,1,3)
        base += np.where(seg[:, :, None], col, 0.0)
    _grad_cache[key] = base
    return base

# ------------------------------------------------------------- grain/vig ---
_grain_noise = None
_vig_mask = None
def _grain_and_vig():
    global _grain_noise, _vig_mask
    if _grain_noise is None:
        _grain_noise = np.random.normal(0, 5.0, (H, W, 1)).astype(np.float32)
        yy, xx = _grids()
        d = np.sqrt(((xx - W / 2) / (W / 2)) ** 2 +
                    ((yy - H / 2) / (H / 2)) ** 2)
        _vig_mask = (1 - 0.32 * np.clip(d - 0.5, 0, 1) / 0.5)[..., None].astype(np.float32)
    return _grain_noise, _vig_mask

# ---------------------------------------------------------------- canvas -----
class Canvas:
    """Composites gradient base + additive glows + one vector layer per frame."""

    def __init__(self):
        self.img = np.zeros((H, W, 3), dtype=np.float32)

    # -- gradients / atmospherics ------------------------------------------
    def base_gradient(self, stops):
        self.img[:] = _cached_gradient(stops)

    def glow(self, cx, cy, r, intensity, color):
        """Additive radial glow orb (soft bloom light), bounds-limited."""
        yy, xx = _grids()
        m = int(6 * r)
        x0 = max(0, int(cx) - m); x1 = min(W, int(cx) + m + 1)
        y0 = max(0, int(cy) - m); y1 = min(H, int(cy) + m + 1)
        if x0 >= x1 or y0 >= y1:
            return
        subxx = xx[y0:y1, x0:x1]
        subyy = yy[y0:y1, x0:x1]
        d2 = (subxx - cx) ** 2 + (subyy - cy) ** 2
        mask = np.exp(-d2 / (2 * r * r)) * intensity
        ccol = np.array(color, dtype=np.float32)[None, None]
        self.img[y0:y1, x0:x1] += mask[..., None] * ccol

    def shadow(self, cx, cy, r, intensity):
        """Additive negative glow (toward black) for depth."""
        yy, xx = _grids()
        m = int(6 * r)
        x0 = max(0, int(cx) - m); x1 = min(W, int(cx) + m + 1)
        y0 = max(0, int(cy) - m); y1 = min(H, int(cy) + m + 1)
        if x0 >= x1 or y0 >= y1:
            return
        subxx = xx[y0:y1, x0:x1]
        subyy = yy[y0:y1, x0:x1]
        d2 = (subxx - cx) ** 2 + (subyy - cy) ** 2
        mask = np.exp(-d2 / (2 * r * r)) * intensity
        self.img[y0:y1, x0:x1] -= mask[..., None] * 255.0

    # -- vector geometry (accumulated on a persistent RGBA layer) -----------
    def _layer(self):
        if getattr(self, "_vl", None) is None:
            self._vl = Image.new("RGBA", (W, H), (0, 0, 0, 0))
            self._dr = ImageDraw.Draw(self._vl)
        return self._vl, self._dr

    def poly(self, points, color, blur=0.0):
        vl, dr = self._layer()
        if blur > 0.5:
            tmp = Image.new("RGBA", (W, H), (0, 0, 0, 0))
            tdr = ImageDraw.Draw(tmp)
            tdr.polygon([(int(x), int(y)) for x, y in points],
                        fill=(*color, 255))
            tmp = tmp.filter(ImageFilter.GaussianBlur(blur))
            vl.alpha_composite(tmp)
        else:
            dr.polygon([(int(x), int(y)) for x, y in points],
                       fill=(*color, 255))

    def rect(self, xy, fill, outline=None, width=0, round_=0):
        _, dr = self._layer()
        box = (int(xy[0]), int(xy[1]), int(xy[2]), int(xy[3]))
        col = (*fill, 255)
        oc = (*outline, 255) if outline else None
        if round_:
            dr.rounded_rectangle(box, radius=int(round_), fill=col,
                                 outline=oc, width=width or 1)
        else:
            dr.rectangle(box, fill=col, outline=oc, width=width or 1)

    def line(self, x0, y0, x1, y1, fill, width=1, dots=None):
        _, dr = self._layer()
        if dots:
            for k in range(dots):
                f = k / max(1, dots - 1)
                dr.line([int(x0 + (x1 - x0) * f), int(y0 + (y1 - y0) * f),
                         int(x0 + (x1 - x0) * f), int(y0 + (y1 - y0) * f)],
                        fill=(*fill, 255), width=width)
        else:
            dr.line([int(x0), int(y0), int(x1), int(y1)],
                    fill=(*fill, 255), width=width)

    def text(self, pos, s, size, fill, kind="bold", anchor="mm",
             alpha=1.0, tracking=0.0, stroke=0, stroke_fill=(0, 0, 0)):
        """Draw text, optionally with letter-spacing (tracking)."""
        if alpha <= 0.01 or not s:
            return
        _, dr = self._layer()
        fnt = font(kind, size)
        cx, cy = pos
        a = int(255 * max(0.0, min(1.0, alpha)))
        if tracking:
            widths = [dr.textlength(ch, font=fnt) + tracking for ch in s]
            total = sum(widths) - tracking if s else 0
            if anchor.endswith("m"):
                x = cx - total / 2
                y = cy - size / 2
            else:
                x, y = cx, cy
            for ch, w in zip(s, widths):
                tx = x + w / 2
                ty = y + size / 2 if anchor.startswith("m") else y
                if stroke and stroke_fill:
                    dr.text((tx, ty), ch, font=fnt, fill=(*fill, a),
                            anchor="mm", stroke_width=stroke,
                            stroke_fill=(*stroke_fill, 255))
                else:
                    dr.text((tx, ty), ch, font=fnt, fill=(*fill, a),
                            anchor="mm")
                x += w
        else:
            if stroke:
                dr.text((cx, cy), s, font=fnt, fill=(*fill, a), anchor=anchor,
                        stroke_width=stroke, stroke_fill=(*stroke_fill, 255))
            else:
                dr.text((cx, cy), s, font=fnt, fill=(*fill, a), anchor=anchor)

    def finish(self, grain=5.0, vig=0.32):
        """Composite vector layer over base, add grain+vignette -> uint8."""
        if getattr(self, "_vl", None) is not None:
            rgba = np.asarray(self._vl, dtype=np.float32)
            alpha = rgba[..., 3:4] / 255.0
            rgb = rgba[..., :3]
            self.img = self.img * (1 - alpha) + rgb * alpha
        noise, vmask = _grain_and_vig()
        arr = self.img + noise
        arr = arr * vmask
        return np.clip(arr, 0, 255).astype(np.uint8)
