#!/usr/bin/env python3
"""Generate 8 images with patience for 429 rate limits. Logs to gen_images.log"""
import sys
import time

import cf_ai as cf
from build_video import SECTIONS

LOG = open("gen_images.log", "a", buffering=1)


def log(msg):
    print(msg, file=LOG)

def image_with_retry(prompt, out, attempts=30, base=15):
    for i in range(attempts):
        try:
            cf.image(prompt, out)
            log(f"OK {out}")
            return True
        except Exception as e:
            code = getattr(e, "code", 0)
            log(f"try {i+1} {out}: http={code} {e}")
            if code == 429:
                time.sleep(base * (i + 1))  # 15,30,45,...s
            else:
                time.sleep(5)
    log(f"FAILED {out}")
    return False


def main():
    log("=== gen_images start ===")
    ok = 0
    for sid, _, prompt in SECTIONS:
        out = f"img/{sid}.png"
        if image_with_retry(prompt, out):
            ok += 1
        # extra cooldown between sections to respect rate limit
        time.sleep(10)
    log(f"done: {ok}/8 images")
    log("=== gen_images end ===")


if __name__ == "__main__":
    main()
