#!/usr/bin/env python3
"""Local procedural cinematic still generator.

Creates attractive, filmic 1024x1024 source images using only numpy + Pillow:
deep multi-stop gradients, soft bokeh orbs, vignette, film grain, and an
optional thematic silhouette. Designed to look like intentional cinematic
frames (teal-and-orange grade) rather than flat clip-art. The output is used
as a still that ffmpeg animates with Ken Burns motion at assembly time.

Usage:
    python3 local_art.py <theme> <out.png>   # theme: emissive|window|fire|phone|chair|road|scale|exit
"""
import math
import os
import sys

import numpy as np
from PIL import Image, ImageDraw, ImageFilter

W = H = 1024


def _blob(w, h, cx, cy, radius, intensity, r, g, b):
    """Add a soft radial-glow orb (bokeh light) onto the RGB array."""
    yy, xx = np.mgrid[0:h, 0:w]
    d2 = (xx - cx) ** 2 + (yy - cy) ** 2
    mask = np.exp(-d2 / (2 * radius * radius)) * intensity
    img = np.dstack([r, g, b])  # (h,w,3)
    return mask[..., None] * img


def _gradient(w, h, stops):
    """Vertical gradient: stops = [(pos0..1, (r,g,b)), ...]."""
    yy = (np.arange(h) / (h - 1))[..., None]  # (h,1)
    base = np.zeros((h, w, 3), dtype=np.float64)
    for i in range(len(stops) - 1):
        p0, c0 = stops[i]
        p1, c1 = stops[i + 1]
        t = np.clip((yy - p0) / (p1 - p0), 0, 1)
        # restict to the segment, keep broadcastable
        seg = (yy >= p0) & (yy <= p1)
        col = np.array(c0, dtype=float) * (1 - t) + np.array(c1, dtype=float) * t
        base += np.where(seg, col, 0.0)
    return base


def _grain(arr, amount=6.0):
    noise = np.random.normal(0, amount, arr.shape[:2])[..., None]
    return np.clip(arr + noise, 0, 255)


def _vignette(arr, strength=0.35):
    yy, xx = np.mgrid[0:H, 0:W]
    d = np.sqrt(((xx - W / 2) / (W / 2)) ** 2 + ((yy - H / 2) / (H / 2)) ** 2)
    v = 1 - strength * np.clip(d - 0.5, 0, 1) / 0.5
    return arr * v[..., None]


def _silhouette(arr, points, color=(5, 8, 12), blur=8):
    """Draw a filled polygon silhouette (thematic foreground shape)."""
    im = Image.fromarray(np.clip(arr, 0, 255).astype(np.uint8))
    dr = ImageDraw.Draw(im)
    dr.polygon(points, fill=color)
    im = im.filter(ImageFilter.GaussianBlur(blur))
    return np.asarray(im, dtype=np.float64)


def render(theme, out):
    # Each theme: gradient stops + optional bokeh orbs + thematic silhouette.
    if theme == "emissive":
        stops = [(0.0, (12, 24, 44)), (0.5, (28, 60, 96)), (1.0, (6, 10, 22))]
        arr = _gradient(W, H, stops)
        for (cx, cy, rad, it, r, g, b) in [
            (300, 300, 140, 0.55, 255, 178, 90),   # warm key / doorway glow
            (820, 520, 120, 0.35, 90, 140, 255),
            (760, 220, 80, 0.28, 255, 200, 120),
        ]:
            arr = arr + _blob(W, H, cx, cy, rad, it, r, g, b)
        arr = _silhouette(arr, [(0, 900), (420, 620), (500, 620), (1024, 880), (1024, 1024), (0, 1024)])
    elif theme == "window":
        stops = [(0.0, (20, 30, 44)), (0.55, (35, 52, 70)), (1.0, (10, 16, 28))]
        arr = _gradient(W, H, stops)
        # bright "greener" distant landscape inside a window frame
        arr = arr + _blob(W, H, 620, 360, 200, 0.5, 120, 200, 120)
        arr = arr + _blob(W, H, 720, 420, 120, 0.4, 160, 220, 130)
        arr = _silhouette(arr, [(560, 420), (700, 420), (700, 700), (560, 700)], color=(4, 6, 10), blur=2)
        arr = _silhouette(arr, [(300, 0), (500, 0), (500, 1024), (300, 1024)], color=(6, 9, 16), blur=6)
    elif theme == "fire":
        stops = [(0.0, (40, 12, 8)), (0.5, (20, 8, 6)), (1.0, (4, 4, 8))]
        arr = _gradient(W, H, stops)
        for (cx, cy, rad, it, r, g, b) in [
            (520, 700, 180, 0.6, 255, 120, 40),    # the fire
            (460, 700, 120, 0.5, 255, 180, 70),
            (580, 720, 100, 0.4, 255, 90, 30),
            (820, 320, 150, 0.5, 255, 160, 60),    # gasoline pour ("hand")
        ]:
            arr = arr + _blob(W, H, cx, cy, rad, it, r, g, b)
        arr = _silhouette(arr, [(700, 240), (940, 240), (1024, 620), (1024, 1024), (700, 1024)], color=(6, 5, 10), blur=10)
    elif theme == "phone":
        stops = [(0.0, (8, 10, 16)), (1.0, (18, 22, 32))]
        arr = _gradient(W, H, stops)
        # glowing phone screen
        arr = arr + _blob(W, H, 512, 560, 260, 0.5, 60, 180, 255)
        arr = arr + _blob(W, H, 512, 560, 120, 0.4, 200, 240, 255)
        # phone body silhouette
        arr = _silhouette(arr, [(382, 380), (642, 380), (642, 760), (382, 760)],
                          color=(10, 14, 22), blur=2)
        # green "online" dot
        arr = _blob(W, H, 512, 560, 18, 0.9, 80, 255, 120)
    elif theme == "chair":
        stops = [(0.0, (4, 5, 10)), (0.5, (12, 14, 24)), (1.0, (3, 4, 8))]
        arr = _gradient(W, H, stops)
        # single harsh overhead spotlight
        arr = arr + _blob(W, H, 512, 120, 300, 0.5, 255, 250, 235)
        arr = arr + _blob(W, H, 512, 120, 120, 0.4, 255, 255, 250)
        # bowed silhouette
        arr = _silhouette(arr, [(360, 520), (470, 520), (470, 640), (560, 640),
                                (560, 520), (664, 520), (664, 780), (360, 780)],
                          color=(6, 7, 12), blur=6)
    elif theme == "road":
        stops = [(0.0, (90, 70, 60)), (0.5, (50, 40, 44)), (1.0, (12, 14, 22))]
        arr = _gradient(W, H, stops)
        arr = arr + _blob(W, H, 512, 300, 260, 0.45, 255, 160, 90)  # dusk sun
        # road
        arr = _silhouette(arr, [(470, 1024), (554, 1024), (600, 0), (424, 0)],
                          color=(6, 6, 10), blur=4)
        # phone booth silhouette at roadside
        arr = _silhouette(arr, [(250, 480), (330, 480), (330, 640), (250, 640)],
                          color=(8, 9, 14), blur=3)
    elif theme == "scale":
        stops = [(0.0, (30, 28, 22)), (0.6, (18, 17, 14)), (1.0, (8, 8, 8))]
        arr = _gradient(W, H, stops)
        for (cx, cy, rad, it, r, g, b) in [
            (380, 520, 120, 0.5, 220, 190, 130),   # house pan (gold)
            (680, 520, 120, 0.5, 220, 190, 130),   # coins pan
        ]:
            arr = arr + _blob(W, H, cx, cy, rad, it, r, g, b)
        arr = _silhouette(arr, [(70, 200), (170, 140), (200, 140), (300, 200),
                                (300, 220), (72, 220)], color=(10, 9, 8), blur=2)  # scale arm
        arr = _silhouette(arr, [(180, 220), (210, 920), (166, 920)], color=(8, 8, 8), blur=3)
        arr = _silhouette(arr, [(300, 220), (330, 920), (280, 920)], color=(8, 8, 8), blur=3)
    elif theme == "exit":
        stops = [(0.0, (6, 8, 14)), (0.5, (14, 18, 30)), (1.0, (40, 50, 70))]
        arr = _gradient(W, H, stops)
        arr = arr + _blob(W, H, 1000, 480, 320, 0.7, 255, 200, 110)  # bright doorway
        # corridor silhouette converging to the light
        arr = _silhouette(arr, [(0, 0), (560, 360), (560, 664), (0, 1024)], color=(5, 6, 12), blur=8)
        arr = _silhouette(arr, [(1024, 0), (700, 360), (700, 664), (1024, 1024)], color=(5, 6, 12), blur=8)
        # walking figure
        arr = _silhouette(arr, [(700, 560), (760, 560), (770, 800), (690, 800)], color=(12, 14, 24), blur=3)
    else:
        raise SystemExit(f"unknown theme {theme}")

    arr = _grain(arr)
    arr = _vignette(arr)
    im = Image.fromarray(np.clip(arr, 0, 255).astype(np.uint8))
    im.save(out)
    return out


if __name__ == "__main__":
    render(sys.argv[1], sys.argv[2])
