# ShelterSave-Studio
# Copyright (C) 2026 Lev Burmistrov (Mahiron-hq)
# SPDX-License-Identifier: GPL-3.0-or-later
"""Crafting costs, computed the way the game builds its recipes."""

import math
import struct


def f32(x):
    return struct.unpack("<f", struct.pack("<f", x))[0]


def craft_seconds(days):
    """CraftParameter.GetBaseCraftTime: whole days, hours, minutes and seconds, as float32."""
    d = math.floor(days)
    hours = f32((f32(days) - d) * 24)
    h = math.floor(hours)
    mins = f32((hours - h) * 60)
    m = math.floor(mins)
    s = math.floor(f32((mins - m) * 60))
    return d * 86400 + h * 3600 + m * 60 + s

RARITY_KEY = {2: "common", 3: "rare", 4: "legendary"}
JUNK_KEYS = ("m_commonJunkCount", "m_rareJunkCount", "m_legendaryJunkCount")


class Crafting:
    """Port of the game's CraftParameter and RecipeList.BuildRecipe."""

    def __init__(self, items):
        self.mapping = items["m_craftParameters"]["m_itemMappingDatas"]
        self.junks = items["m_junksList"]

    def junk(self, component, rarity):
        return next((j for j in self.junks if j["m_itemRarity"] == rarity and j["m_linkedComponent"] == component),
                    None)

    def find(self, kind, item):
        for m in self.mapping:
            if kind == "Weapon":
                avg = (item["m_DamageMin"] + item["m_DamageMax"]) / 2
                if m["m_weaponAverageDamageMax"] >= avg and m["m_weaponAverageDamageMin"] < avg:
                    return m
            elif kind == "Outfit":
                total = sum(v["Value"] for v in item["m_specialStats"].values())
                if m["m_outfitBonus"] == min(total, 8):
                    return m
            elif kind == "Theme" and m["m_themeId"] == item["m_codeId"]:
                return m
        return self.mapping[-1]

    def recipe(self, kind, item):
        r = RARITY_KEY.get(item["m_itemRarity"])
        if r is None or not item.get("m_canBeCrafted"):
            return None
        m = self.find(kind, item)
        caps = m[f"m_{r}ItemCraftCost"]
        seconds = craft_seconds(m[f"m_{r}ItemCraftTimeDayCount"])
        ing = self.theme_ingredients(item, m) if kind == "Theme" else self.ingredients(item, m[f"m_{r}ItemJunkCount"])
        if ing is None:
            return None
        return {"caps": caps, "junk": ing, "time": seconds}

    def theme_ingredients(self, t, m):
        out = []
        for i, key in enumerate(("Common", "Common", "Common", "Rare", "Rare")):
            comp = t[f"m_{key.lower()}Component0{i + 1 if i < 3 else i - 2}"]
            if comp == -1:
                continue
            counts = m[f"m_{key.lower()}Component0{i + 1 if i < 3 else i - 2}Count"]
            n, rarity = (counts["m_commonJunkCount"], 2) if i < 3 else (counts["m_rareJunkCount"], 3)
            j = self.junk(comp, rarity)
            if n > 0 and j is not None:
                out.append([j["m_JunkId"], n])
        return out

    def ingredients(self, item, counts):
        arr = [counts[k] for k in JUNK_KEYS]
        if item["m_primaryComponent"] == -1 or all(arr):
            return None
        total = sum(arr)
        comps = [item["m_primaryComponent"], item["m_secondaryComponent"], item["m_tertiaryComponent"]]
        while True:
            grid = [[0] * 3 for _ in range(3)]
            used = 0
            n_comp = 1 + (comps[1] != -1) + (comps[2] != -1)
            first = second = 0
            for k in (2, 1, 0):
                if arr[k] > 0:
                    if first == 0:
                        first = arr[k]
                    else:
                        second = arr[k]
            split = [0, 0, 0]
            if n_comp == 1:
                split = [total, 0, 0]
            elif n_comp == 2:
                if second == 0:
                    split[0] = -(-total // 2)
                    split[1] = total - split[0]
                elif first < second:
                    split[0] = first + math.ceil(second - total / 2)
                    split[1] = total - split[0]
                elif first == second or first == second + 1:
                    split = [first, second, 0]
                else:
                    split[0] = -(-total // 2)
                    split[1] = total - split[0]
            else:
                if second == 0:
                    split[0] = -(-total // 3)
                    split[1] = -(-(total - split[0]) // 2)
                    split[2] = total - split[0] - split[1]
                elif first == 1 and second == 1:
                    split = [first, second, 0]
                elif first < second:
                    split = [first, -(-second // 2), 0]
                    split[2] = second - split[1]
                else:
                    split = [-(-first // 2), 0, second]
                    split[1] = first - split[0]
            taken_c = [0, 0, 0]
            taken_r = [0, 0, 0]
            for r in (2, 1, 0):
                for c in range(3):
                    grid[c][r] = min(arr[r] - taken_r[r], split[c] - taken_c[c])
                    if grid[c][r] > 0:
                        taken_c[c] += grid[c][r]
                        taken_r[r] += grid[c][r]
                        used += 1
            ok = True
            for c in range(3):
                for r in range(3):
                    if ok and grid[c][r] > 0 and comps[c] != -1 and self.junk(comps[c], r + 2) is None:
                        ok = False
                        comps = comps[:c] + comps[c + 1:] + [-1]
                if not ok:
                    break
            if ok:
                break
        out = []
        for c in range(3):
            for r in range(3):
                if grid[c][r] > 0:
                    j = self.junk(comps[c], r + 2)
                    if j is None:
                        return None
                    out.append([j["m_JunkId"], grid[c][r]])
        return out
