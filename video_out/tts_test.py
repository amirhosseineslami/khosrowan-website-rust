#!/usr/bin/env python3
"""Test Cloudflare aura-2-en TTS (async) - generate one narration clip."""
import json
import sys
import time
import urllib.request
import base64

ACC = "2c38ed3a6fd23faf8181d6f4397d07a3"
TOKEN = open("/home/amir/Projects/personal-site/video_out/.cf_token").read().strip()


def api(path, method="GET", body=None, raw=False):
    data = json.dumps(body).encode() if body else None
    req = urllib.request.Request(
        f"https://api.cloudflare.com/client/v4{path}",
        method=method, data=data,
        headers={"Authorization": f"Bearer {TOKEN}", "Content-Type": "application/json"},
    )
    with urllib.request.urlopen(req) as r:
        return r.read() if raw else json.load(r)


def run(model, payload):
    return api(f"/accounts/{ACC}/ai/run/{model}", method="POST", body=payload)


text = sys.argv[1] if len(sys.argv) > 1 else "Never beg a woman who's walking out of the relationship."
out = sys.argv[2] if len(sys.argv) > 2 else "audio/tts_test.mp3"

# Try direct (sync) first - aura returns raw audio bytes
try:
    data = api(f"/accounts/{ACC}/ai/run/@cf/deepgram/aura-2-en", method="POST",
               body={"text": text, "voice": "aura-2-en"}, raw=True)
    # check if it's JSON error vs binary
    if data[:1] == b'{':
        d = json.loads(data)
        print("sync json success:", d.get("success"), "errors:", d.get("errors"))
        print(json.dumps(d, indent=2)[:1000])
    else:
        open(out, "wb").write(data)
        print("wrote raw audio", out, len(data), "bytes")
except Exception as e:
    print("sync error:", e)
