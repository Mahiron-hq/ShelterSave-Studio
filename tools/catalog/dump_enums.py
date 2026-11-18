# ShelterSave-Studio
# Copyright (C) 2026 Lev Burmistrov (Mahiron-hq)
# SPDX-License-Identifier: GPL-3.0-or-later
"""Extracts enum tables needed by build_catalog.py from ILSpy output of Assembly-CSharp.dll."""

import argparse
import json
import re
from pathlib import Path

ENUMS = ["EPetBreed", "EBonusEffect", "ERoomType", "ERoomClass", "ERoomEmergencyMode", "EEnemyType", "ESpecialStat", "EReward",
         "ESpecialTheme"]


def parse_enum(text, name):
    m = re.search(r"enum\s+" + name + r"\b[^{]*\{(.*?)\}", text, re.S)
    if not m:
        return None
    out, cur = {}, -1
    for part in m.group(1).split(","):
        part = re.sub(r"//.*", "", part).strip()
        if not part or part.startswith("["):
            continue
        if "=" in part:
            k, v = (x.strip() for x in part.split("=", 1))
            cur = {"int.MinValue": -2147483648, "int.MaxValue": 2147483647}.get(v) if not re.fullmatch(r"-?(0x)?[0-9a-fA-F]+", v) else int(v, 0)
        else:
            k = part
            cur += 1
        out[str(cur)] = k
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--src", required=True)
    ap.add_argument("--out", required=True)
    a = ap.parse_args()
    res = {}
    for name in ENUMS:
        for f in Path(a.src).rglob(name + ".cs"):
            e = parse_enum(f.read_text(encoding="utf-8", errors="replace"), name)
            if e:
                res[name] = e
                break
        if name not in res:
            raise SystemExit(f"enum {name} not found")
    Path(a.out).write_text(json.dumps(res, indent=1) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
