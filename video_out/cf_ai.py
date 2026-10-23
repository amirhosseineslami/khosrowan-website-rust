#!/usr/bin/env python3
"""Cloudflare Workers AI proxy helpers - image gen, TTS, chat, whisper.

Usage:
  cf_ai.py chat <model> <prompt>
  cf_ai.py image <prompt> <outfile> [size]     # FLUX klein 4b
  cf_ai.py tts <text> <outfile> [voice]        # aura-2-en (returns mp3)
  cf_ai.py whisper <audiofile>                 # returns transcript JSON
"""
import base64
import json
import sys
import urllib.request

ACC = "2c38ed3a6fd23faf8181d6f4397d07a3"
TOKEN = open("/home/amir/Projects/personal-site/video_out/.cf_token").read().strip()
BASE = "https://api.cloudflare.com/client/v4"


def api(path, method="GET", body=None, raw=False, form=None):
    if form is not None:
        # multipart form data
        boundary = "----hermescfboundary"
        parts = []
        for k, v in form.items():
            parts.append(f"--{boundary}\r\nContent-Disposition: form-data; name=\"{k}\"\r\n\r\n{v}\r\n")
        data = ("".join(parts) + f"--{boundary}--\r\n").encode()
        req = urllib.request.Request(f"{BASE}{path}", method=method, data=data,
            headers={"Authorization": f"Bearer {TOKEN}",
                     "Content-Type": f"multipart/form-data; boundary={boundary}"})
    else:
        data = json.dumps(body).encode() if body else None
        req = urllib.request.Request(f"{BASE}{path}", method=method, data=data,
            headers={"Authorization": f"Bearer {TOKEN}", "Content-Type": "application/json"})
    with urllib.request.urlopen(req) as r:
        return r.read() if raw else json.load(r)


def chat(model, prompt):
    d = api(f"/accounts/{ACC}/ai/run/{model}", method="POST",
            body={"messages": [{"role": "user", "content": prompt}]})
    if d.get("success"):
        return d["result"].get("response", "")
    raise RuntimeError(json.dumps(d.get("errors")))


def image(prompt, outpath, size="1024x1024", steps=4):
    w, h = size.lower().split("x")
    form = {"prompt": prompt, "size": f"{w},{h}", "steps": str(steps)}
    d = api(f"/accounts/{ACC}/ai/run/@cf/black-forest-labs/flux-2-klein-4b",
            method="POST", form=form)
    if d.get("success") and d.get("result", {}).get("image"):
        open(outpath, "wb").write(base64.b64decode(d["result"]["image"]))
        return outpath
    raise RuntimeError(json.dumps(d.get("errors")))


def tts(text, outpath, voice="aura-2-en"):
    data = api(f"/accounts/{ACC}/ai/run/@cf/deepgram/aura-2-en", method="POST",
               body={"text": text, "voice": voice}, raw=True)
    if data[:1] == b'{':  # JSON error
        raise RuntimeError(data.decode())
    open(outpath, "wb").write(data)
    return outpath


def whisper(audiopath):
    data = base64.b64encode(open(audiopath, "rb").read()).decode()
    d = api(f"/accounts/{ACC}/ai/run/@cf/openai/whisper", method="POST",
            body={"audio": data})
    return d.get("result", {})


if __name__ == "__main__":
    cmd = sys.argv[1] if len(sys.argv) > 1 else ""
    if cmd == "chat":
        print(chat(sys.argv[2], sys.argv[3]))
    elif cmd == "image":
        print(image(sys.argv[2], sys.argv[3], sys.argv[4] if len(sys.argv) > 4 else "1024x1024"))
    elif cmd == "tts":
        print(tts(sys.argv[2], sys.argv[3], sys.argv[4] if len(sys.argv) > 4 else "aura-2-en"))
    elif cmd == "whisper":
        print(json.dumps(whisper(sys.argv[2]), indent=2))
    else:
        sys.exit(__doc__)
