#!/usr/bin/env python3
"""Test Gemini image generation via the native API."""
import base64
import json
import os
import sys
import urllib.request

KEY = os.environ["GOOGLE_API_KEY"]
MODEL = "gemini-2.5-flash-image"
URL = f"https://generativelanguage.googleapis.com/v1beta/models/{MODEL}:generateContent?key={KEY}"

def make_image(prompt, out):
    payload = {
        "contents": [{"parts": [{"text": prompt}]}],
        "generationConfig": {"responseModalities": ["IMAGE", "TEXT"]}
    }
    req = urllib.request.Request(URL, data=json.dumps(payload).encode(),
                                 headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(req) as r:
        d = json.load(r)
    parts = d["candidates"][0]["content"]["parts"]
    for p in parts:
        if "inlineData" in p:
            data = p["inlineData"]["data"]
            mime = p["inlineData"]["mimeType"]
            ext = "png" if "png" in mime else "jpg"
            with open(out, "wb") as f:
                f.write(base64.b64decode(data))
            return f"wrote {out} ({mime}, {len(data)} b64)"
    return json.dumps(d, indent=2)[:600]

if __name__ == "__main__":
    out = sys.argv[2] if len(sys.argv) > 2 else "img/gemini_test.png"
    print(make_image(sys.argv[1], out))
