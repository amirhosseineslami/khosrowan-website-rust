#!/usr/bin/env python3
"""Render Never Beg v2 using the semantic motion engine (scenes_new)."""
import os
import subprocess
import sys

import numpy as np
from scenes_new import render_scene

FPS = 30
W, H = 1920, 1080
GAP = 0.5
SECTIONS = [f"s{i:02d}" for i in range(1, 9)]


def probe_dur(path):
    out = subprocess.run(
        ["ffprobe", "-v", "error", "-show_entries", "format=duration",
         "-of", "csv=p=0", path],
        capture_output=True, text=True)
    return float(out.stdout.strip())


def render_section(sid, dur, out_path):
    frames = int(round(FPS * dur))
    cmd = [
        "ffmpeg", "-y",
        "-f", "rawvideo", "-pix_fmt", "rgb24",
        "-s", f"{W}x{H}", "-r", str(FPS),
        "-i", "-",
        "-c:v", "libx264", "-preset", "medium", "-crf", "18",
        "-pix_fmt", "yuv420p", "-movflags", "+faststart",
        out_path,
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


def main():
    os.makedirs("seg/v2", exist_ok=True)
    durs = {s: probe_dur(f"audio/{s}.mp3") for s in SECTIONS}
    print("durations:", {k: round(v, 2) for k, v in durs.items()})
    segs = []
    for sid in SECTIONS:
        out = f"seg/v2/{sid}.mp4"
        print(f"render {sid} {durs[sid]:.2f}s ...", flush=True)
        render_section(sid, durs[sid], out)
        segs.append(out)
    # concat (re-encode via concat filter)
    inputs = []
    for s in segs:
        inputs += ["-i", s]
    fc = "".join(f"[{i}:v]" for i in range(len(segs)))
    fc += f"concat=n={len(segs)}:v=1:a=0[v]"
    subprocess.run([
        "ffmpeg", "-y", *inputs, "-filter_complex", fc,
        "-map", "[v]", "-c:v", "libx264", "-crf", "18",
        "-pix_fmt", "yuv420p", "-movflags", "+faststart",
        "seg/v2/video_only.mp4",
    ], check=True, capture_output=True)
    print("video_only done")
    # mux narration
    subprocess.run([
        "ffmpeg", "-y",
        "-i", "seg/v2/video_only.mp4",
        "-i", "audio/narration_full.mp3",
        "-map", "0:v", "-map", "1:a",
        "-c:v", "copy", "-c:a", "aac", "-b:a", "192k",
        "-shortest", "-movflags", "+faststart",
        "never_beg_v2.mp4",
    ], check=True, capture_output=True)
    print("final: never_beg_v2.mp4")


if __name__ == "__main__":
    main()
