#!/usr/bin/env python3
"""Test OpenAI image generation via the native API."""
import base64
import json
import os
import sys
import urllib.error
import urllib.request

KEY = os.environ["OPENAI_API_KEY"]
MODEL = "gpt-image-1-mini"  # cheap + fast

def make_image(prompt, out, model=MODEL, size="1024x1024"):
    payload = {"model": model, "prompt": prompt, "size": size, "n": 1}
    req = urllib.request.Request("https://api.openai.com/v1/images/generations",
                                 data=json.dumps(payload).encode(),
                                 headers={"Authorization": f"Bearer {KEY}",
                                          "Content-Type": "application/json"})
    try:
        with urllib.request.urlopen(req) as r:
            d = json.load(r)
    except urllib.error.HTTPError as e:
        body = e.read().decode()
        return f"HTTP {e.code}: {body[:300]}"
    item = d["data"][0]
    if "b64_json" in item:
        with open(out, "wb") as f:
            f.write(base64.b64decode(item["b64_json"]))
        return f"wrote {out} (b64 json)"
    if "url" in item:
        urllib.request.urlretrieve(item["url"], out)
        return f"wrote {out} (from url)"
    return json.dumps(d, indent=2)[:600]

if __name__ == "__main__":
    out = sys.argv[2] if len(sys.argv) > 2 else "img/openai_test.png"
    size = sys.argv[3] if len(sys.argv) > 3 else "1024x1024"
    print(make_image(sys.argv[1], out, size=size))
