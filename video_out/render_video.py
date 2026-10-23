#!/usr/bin/env python3
"""Render the "Never Beg" motion video.

Pipes the programmatic animated scenes straight into ffmpeg (raw RGB via
stdin), encodes each section as H.264, concatenates, then muxes the narration
audio with 0.5s gaps so each scene lands on its spoken section.

Usage:
    python3 render_video.py            # full render (uses audio durations)
    python3 render_video.py --s s01:6  # render just scene s01, 6s, for preview
"""
import json
import os
import subprocess
import sys

import numpy as np
from PIL import Image
from scenes import render_scene

FPS = 30
W, H = 1920, 1080
GAP = 0.5  # seconds between sections in the narration track
SECTIONS = ["s01", "s02", "s03", "s04", "s05", "s06", "s07", "s08"]


def probe_dur(path):
    out = subprocess.run(
        ["ffprobe", "-v", "error", "-show_entries", "format=duration",
         "-of", "csv=p=0", path],
        capture_output=True, text=True)
    return float(out.stdout.strip())


def section_durations():
    durs = {}
    for sid in SECTIONS:
        durs[sid] = probe_dur(f"audio/{sid}.mp3")
    return durs


def render_section(sid, dur, tmp_pipe):
    """Render one section's frames and feed them to the ffmpeg pipe.

    Returns the subprocess.Popen for the encoder.
    """
    frames = int(round(FPS * dur))
    cmd = [
        "ffmpeg", "-y",
        "-f", "rawvideo", "-pix_fmt", "rgb24",
        "-s", f"{W}x{H}", "-r", str(FPS),
        "-i", "-",
        "-c:v", "libx264", "-preset", "medium", "-crf", "18",
        "-pix_fmt", "yuv420p", "-movflags", "+faststart",
        tmp_pipe,
    ]
    proc = subprocess.Popen(cmd, stdin=subprocess.PIPE,
                            stdout=subprocess.DEVNULL,
                            stderr=subprocess.DEVNULL)
    for f in range(frames):
        t = f / FPS
        frame = render_scene(sid, t)
        proc.stdin.write(frame.tobytes())
    proc.stdin.close()
    proc.wait()
    return proc


def concat_segments(segs, out_video):
    lst = "seg/concat.txt"
    with open(lst, "w") as fh:
        for s in segs:
            fh.write(f"file '{s}'\n")
    # Concat with re-encode (different motion streams) — use concat filter.
    inputs = []
    for s in segs:
        inputs += ["-i", s]
    fc = "".join(f"[{i}:v]" for i in range(len(segs)))
    fc += f"concat=n={len(segs)}:v=1:a=0[v]"
    subprocess.run([
        "ffmpeg", "-y", *inputs, "-filter_complex", fc,
        "-map", "[v]", "-c:v", "libx264", "-crf", "18",
        "-pix_fmt", "yuv420p", "-movflags", "+faststart", out_video,
    ], check=True, capture_output=True)


def build_narration(durs, out):
    """Concatenate section MP3s with 0.5s silence gaps."""
    inputs = []
    for sid in SECTIONS:
        inputs += ["-i", f"audio/{sid}.mp3"]
    filters = []
    start = 0.0
    for i, sid in enumerate(SECTIONS):
        delay_ms = int(round(start * 1000))
        filters.append(
            f"[{i}:a]aresample=24000,adelay={delay_ms}|{delay_ms}[a{i}]")
        start += durs[sid] + GAP
    amix = "".join(f"[a{i}]" for i in range(len(SECTIONS)))
    filters.append(f"{amix}amix=inputs={len(SECTIONS)}:normalize=0[out]")
    fc = ";".join(filters)
    subprocess.run([
        "ffmpeg", "-y", *inputs, "-filter_complex", fc,
        "-map", "[out]", "-c:a", "libmp3lame", "-q:a", "2", out,
    ], check=True, capture_output=True)


def main():
    if len(sys.argv) > 1 and sys.argv[1] == "--s":
        sid = sys.argv[2]
        dur = float(sys.argv[3])
        out = f"seg/{sid}_preview.mp4"
        render_section(sid, dur, out)
        print("preview:", out)
        # also dump a few frames for inspection
        os.makedirs("preview", exist_ok=True)
        for f in (1, int(dur * FPS / 2), int(dur * FPS) - 1):
            frame = render_scene(sid, f / FPS)
            Image.fromarray(frame).save(f"preview/{sid}_f{f}.png")
        return

    durs = section_durations()
    print("section durations:", {k: round(v, 2) for k, v in durs.items()})

    os.makedirs("seg", exist_ok=True)
    segs = []
    for sid in SECTIONS:
        out_seg = f"seg/{sid}.mp4"
        print(f"rendering {sid} ({durs[sid]:.2f}s)...")
        render_section(sid, durs[sid], out_seg)
        segs.append(out_seg)

    concat_segments(segs, "seg/video_only.mp4")
    print("video_only concat done")

    build_narration(durs, "audio/narration_full.mp3")
    print("narration full done")

    subprocess.run([
        "ffmpeg", "-y",
        "-i", "seg/video_only.mp4",
        "-i", "audio/narration_full.mp3",
        "-map", "0:v", "-map", "1:a",
        "-c:v", "copy", "-c:a", "aac", "-b:a", "192k",
        "-shortest", "-movflags", "+faststart",
        "never_beg.mp4",
    ], check=True, capture_output=True)
    print("final: never_beg.mp4")


if __name__ == "__main__":
    main()
