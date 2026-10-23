#!/usr/bin/env python3
"""Build the "Never Beg" video end-to-end via Cloudflare Workers AI proxy.

Steps:
  1. Generate aura-2-en narration for each of the 8 sections (audio/sNN.mp3).
  2. Probe real durations with ffprobe -> rebuild shotlist timings.
  3. Generate each section's FLUX image (img/sNN.png, 1024x1024).
  4. Crop to 16:9 and render each segment with Ken Burns (zoompan) motion.
  5. Concatenate segments, mux narration audio, export 1920x1080 H.264/AAC.
"""
import json
import subprocess
import sys
import time

import cf_ai as cf

SEC_SECTIONS = [
    ("s01", "(audio/s01.txt)"),
]
# section id -> (text file, prompt)
SECTIONS = [
    ("s01", "audio/s01_intro.txt", "Cinematic film still, 16:9 widescreen. A man standing still in a dark hallway, seen from behind, a woman walking out of frame through an open doorway that glows with warm amber light. High-contrast moody lighting, deep shadows, subtle teal-and-orange grade, shallow depth of field, realistic, subtle film grain, no text, no watermark."),
    ("s02", "audio/s02_r1.txt", "Cinematic film still, 16:9 widescreen. A woman standing at a large window inside a dim modern apartment, looking out across a distant brighter green landscape on the horizon, a faint knowing smirk. Metaphor for the grass is greener. High-contrast moody lighting, deep shadows, subtle teal-and-orange grade, shallow depth of field, realistic, subtle film grain, no text, no watermark."),
    ("s03", "audio/s03_r2.txt", "Cinematic film still, 16:9 widescreen. A small fire burning low on the floor of a modern living room, a hand frozen mid-motion about to pour a gallon of gasoline onto it, dramatic chiaroscuro lighting, thriller mood, deep shadows, subtle teal-and-orange grade, shallow depth of field, realistic, subtle film grain, no text, no watermark."),
    ("s04", "audio/s04_r3.txt", "Cinematic film still, 16:9 widescreen. A smartphone lying face-up on a dark wooden table, glowing screen showing a message thread left on read, a single green online indicator dot, deep shadows, minimal military-dark composition, high-contrast moody lighting, subtle teal-and-orange grade, shallow depth of field, realistic, subtle film grain, no readable large text, no watermark."),
    ("s05", "audio/s05_crit.txt", "Cinematic film still, 16:9 widescreen. A man seated alone on a plain chair under one harsh overhead spotlight, crushed posture, head bowed, surrounded by dense darkness, psychological thriller still, high-contrast moody lighting, deep shadows, subtle teal-and-orange grade, shallow depth of field, realistic, subtle film grain, no text, no watermark."),
    ("s06", "audio/s06_comeback.txt", "Cinematic film still, 16:9 widescreen. An empty winding road at dusk stretching to the horizon, a solitary vintage phone booth standing at the roadside, very long shadows, loneliness and waiting, high-contrast moody lighting, subtle teal-and-orange grade, shallow depth of field, realistic, subtle film grain, no text, no watermark."),
    ("s07", "audio/s07_lesson.txt", "Cinematic film still, 16:9 widescreen. An old brass balance scale on a desk in a dim accounting room, one pan holds a miniature house model, the other pan holds a pile of gold coins and folded banknotes, careful dramatic lighting, deep shadows, subtle teal-and-orange grade, shallow depth of field, realistic, subtle film grain, no text, no watermark."),
    ("s08", "audio/s08_close.txt", "Cinematic film still, 16:9 widescreen. A confident man walking away from camera toward a bright doorway of warm light at the end of a long dark corridor, upright strong posture, silhouette with strong rim light, dawn glowing outside, triumphant and calm, high-contrast moody lighting, deep shadows, subtle teal-and-orange grade, shallow depth of field, realistic, subtle film grain, no text, no watermark."),
]


def probe_dur(path):
    out = subprocess.run(
        ["ffprobe", "-v", "error", "-show_entries", "format=duration",
         "-of", "csv=p=0", path],
        capture_output=True, text=True)
    return float(out.stdout.strip())


def step1_tts():
    """Generate narration audio for each section; return list of durations."""
    durs = []
    for sid, txtfile, _ in SECTIONS:
        text = open(txtfile).read().strip()
        out = f"audio/{sid}.mp3"
        cf.tts(text, out)
        d = probe_dur(out)
        durs.append(d)
        print(f"{sid}: {d:.2f}s -> {out}")
        time.sleep(1)  # be polite to the queue
    return durs


def step2_images():
    """Generate each section's image; return list of paths."""
    paths = []
    for sid, _, prompt in SECTIONS:
        out = f"img/{sid}.png"
        cf.image(prompt, out)
        paths.append(out)
        print(f"{sid}: image -> {out}")
        time.sleep(1)
    return paths


def step3_assemble(durs):
    """Crop to 16:9, Ken Burns each segment, concatenate, mux audio."""
    fps = 30
    seglist = []
    for i, (sid, _, _) in enumerate(SECTIONS):
        dur = durs[i] + 0.5  # small tail gap between sections
        frames = int(round(fps * dur))
        src = f"img/{sid}.png"
        seg = f"seg/{sid}.mp4"
        # crop 1024x1024 -> 1024x576 (16:9) centered, then zoompan push
        vf = (
            f"scale=1920:1920,crop=1920:1080:0:420,"
            f"zoompan=z='min(zoom+0.0008,1.12)':d={frames}:s=1920x1080:fps={fps}"
        )
        subprocess.run([
            "ffmpeg", "-y", "-loop", "1", "-i", src,
            "-t", f"{dur:.3f}", "-vf", vf,
            "-r", str(fps), "-c:v", "libx264", "-pix_fmt", "yuv420p",
            "-crf", "18", "-preset", "medium", seg,
        ], check=True, capture_output=True)
        seglist.append(seg)
        print(f"{sid}: segment {dur:.2f}s -> {seg}")

    # concatenate segments
    concat_list = "seg/concat.txt"
    with open(concat_list, "w") as f:
        for s in seglist:
            f.write(f"file '{s}'\n")
    video = "seg/video_only.mp4"
    subprocess.run([
        "ffmpeg", "-y", "-f", "concat", "-safe", "0", "-i", concat_list,
        "-c", "copy", video,
    ], check=True, capture_output=True)

    # mux the concatenated narration (8 sections merged with silence gaps)
    # Build one full narration track so it matches video timing.
    narration = "audio/narration_full.mp3"
    build_narration(narration, durs)

    final = "never_beg.mp4"
    subprocess.run([
        "ffmpeg", "-y", "-i", video, "-i", narration,
        "-map", "0:v", "-map", "1:a",
        "-c:v", "copy", "-c:a", "aac", "-b:a", "192k",
        "-shortest", "-movflags", "+faststart", final,
    ], check=True, capture_output=True)
    print("final:", final)


def build_narration(out, durs):
    """Concatenate section MP3s with 0.5s silence between them."""
    inputs = []
    for sid, _, _ in SECTIONS:
        inputs += ["-i", f"audio/{sid}.mp3"]
    # filter: adelay each section so it starts at the right time
    # section start = sum of previous durations + gaps
    filters = []
    start = 0.0
    for i, (sid, _, _) in enumerate(SECTIONS):
        delay_ms = int(round(start * 1000))
        filters.append(
            f"[{i}:a]aresample=24000,adelay={delay_ms}|{delay_ms}[a{i}]")
        start += durs[i] + 0.5
    amix = "".join(f"[a{i}]" for i in range(len(SECTIONS)))
    filters.append(f"{amix}amix=inputs={len(SECTIONS)}:normalize=0[out]")
    fc = ";".join(filters)
    subprocess.run([
        "ffmpeg", "-y", *inputs, "-filter_complex", fc,
        "-map", "[out]", "-c:a", "libmp3lame", "-q:a", "2", out,
    ], check=True, capture_output=True)


def main():
    which = sys.argv[1] if len(sys.argv) > 1 else "all"
    if which in ("all", "tts", "1"):
        print("== STEP 1: TTS narration ==")
        durs = step1_tts()
        # write durations back into shotlist.json
        with open("shotlist.json") as f:
            sl = json.load(f)
        t = 0.0
        for i, e in enumerate(sl):
            e["start"] = round(t, 2)
            e["end"] = round(t + durs[i], 2)
            e["dur"] = round(durs[i], 2)
            t += durs[i] + 0.5
        with open("shotlist.json", "w") as f:
            json.dump(sl, f, ensure_ascii=False, indent=2)
        print("shotlist updated with real durations")
        if which == "tts":
            return
    if which in ("all", "images", "2"):
        print("== STEP 2: images ==")
        step2_images()
        if which == "images":
            return
    if which in ("all", "assemble", "3"):
        print("== STEP 3: assemble ==")
        with open("shotlist.json") as f:
            sl = json.load(f)
        durs = [e["dur"] for e in sl]
        step3_assemble(durs)


if __name__ == "__main__":
    main()
