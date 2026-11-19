# ShelterSave-Studio
# Copyright (C) 2026 Lev Burmistrov (Mahiron-hq)
# SPDX-License-Identifier: GPL-3.0-or-later
"""Builds data/catalog.json from an installed copy of Fallout Shelter (PC).

Only identifiers, numbers and names are taken from the game files.

Usage: python build_catalog.py --game "<game folder>" --out ../../data/catalog.json
Requires: UnityPy, TypeTreeGeneratorAPI (see README.md).
"""

import argparse
import json
import struct
import sys
import zlib
from pathlib import Path

import UnityPy
from UnityPy.helpers.TypeTreeGenerator import TypeTreeGenerator

from build_art import build_art
from craft import Crafting

CATALOG_VERSION = "1.0.0"

PRIM = {
    "int": "i", "unsigned int": "I", "SInt32": "i", "UInt32": "I", "float": "f", "double": "d",
    "SInt64": "q", "UInt64": "Q", "long long": "q", "unsigned long long": "Q", "SInt16": "h",
    "UInt16": "H", "short": "h", "unsigned short": "H", "SInt8": "b", "UInt8": "B", "char": "B",
    "bool": "?", "Type*": "i", "FileSize": "q",
}


class Reader:
    def __init__(self, data):
        self.b = data
        self.p = 0

    def align(self):
        self.p = (self.p + 3) & ~3

    def un(self, f):
        v = struct.unpack_from("<" + f, self.b, self.p)[0]
        self.p += struct.calcsize("<" + f)
        return v


def read_node(n, r):
    # The generated trees label string[] vectors as "string", so vectors are detected by shape.
    t = n.m_Type
    kids = n.m_Children
    is_vec = (
        bool(kids) and kids[0].m_Type == "Array" and len(kids[0].m_Children) == 2
        and not (t == "string" and kids[0].m_Children[1].m_Type == "char")
    )
    if t == "string" and not is_vec:
        ln = r.un("i")
        v = r.b[r.p:r.p + ln].decode("utf-8", "replace")
        r.p += ln
        r.align()
        return v
    if t in PRIM and not is_vec:
        v = r.un(PRIM[t])
        if n.m_MetaFlag & 0x4000:
            r.align()
        return v
    if is_vec:
        cnt = r.un("i")
        if cnt < 0 or cnt > 10_000_000:
            raise ValueError(f"bad count {cnt}")
        sub = kids[0].m_Children[1]
        if sub.m_Type in PRIM:
            f = PRIM[sub.m_Type]
            v = list(struct.unpack_from("<" + f * cnt, r.b, r.p))
            r.p += struct.calcsize("<" + f) * cnt
        else:
            v = [read_node(sub, r) for _ in range(cnt)]
        r.align()
        return v
    out = {}
    for c in kids:
        out[c.m_Name] = read_node(c, r)
    if n.m_MetaFlag & 0x4000:
        r.align()
    return out


class Game:
    def __init__(self, game_dir):
        data = Path(game_dir) / "FalloutShelter_Data"
        self.env = UnityPy.load(str(data / "data.unity3d"))
        self.mono = [o for o in self.env.objects if o.type.name == "MonoBehaviour"]
        self.by_id = {(o.assets_file.name, o.path_id): o for o in self.mono}
        self.cls = {}
        cache = {}
        for o in self.mono:
            d = o.read(check_read=False)
            k = (d.m_Script.m_FileID, d.m_Script.m_PathID, o.assets_file.name)
            if k not in cache:
                try:
                    cache[k] = d.m_Script.read().m_ClassName
                except Exception:
                    cache[k] = "?"
            self.cls[(o.assets_file.name, o.path_id)] = cache[k]
        gen = TypeTreeGenerator(self.env.objects[0].assets_file.unity_version)
        gen.load_local_dll_folder(str(data / "Managed"))
        self.env.typetree_generator = gen
        self.version = (Path(game_dir) / "FalloutShelter_Data" / "StreamingAssets" / "build_info.txt").read_text(
            encoding="utf-8", errors="replace").strip()
        self.game_version = next((p.name for p in Path(game_dir).iterdir() if p.is_file() and p.name[0].isdigit()), "")

    def read(self, o):
        node = o._get_typetree_node()
        r = Reader(o.get_raw_data())
        out = {}
        for c in node.m_Children:
            out[c.m_Name] = read_node(c, r)
        out.pop("m_GameObject", None)
        out.pop("m_Script", None)
        return out

    def of_class(self, name, prefer_shared=True):
        objs = [o for o in self.mono if self.cls[(o.assets_file.name, o.path_id)] == name]
        if prefer_shared:
            shared = [o for o in objs if not o.assets_file.name.startswith("level")]
            objs = shared or objs
        return objs

    def components(self, o):
        """MonoBehaviours on the same GameObject as o."""
        go = o.read(check_read=False).m_GameObject.deref()
        if go is None:
            return []
        out = []
        for c in go.read().m_Component:
            obj = c.component.deref() if hasattr(c, "component") else c.deref()
            if obj is not None and obj.type.name == "MonoBehaviour":
                out.append(obj)
        return out

    def child_components(self, o):
        """MonoBehaviours on the children of o's GameObject."""
        go = o.read(check_read=False).m_GameObject.deref()
        out = []
        for c in go.read().m_Component:
            t = c.component.deref() if hasattr(c, "component") else c.deref()
            if t is None or t.type.name != "Transform":
                continue
            for ch in t.read().m_Children:
                cgo = ch.deref().read().m_GameObject.deref()
                for cc in cgo.read().m_Component:
                    obj = cc.component.deref() if hasattr(cc, "component") else cc.deref()
                    if obj is not None and obj.type.name == "MonoBehaviour":
                        out.append(obj)
        return out

    def deref(self, owner, pptr):
        return self.by_id.get((owner.assets_file.name, pptr["m_PathID"]))

    def path(self, o):
        """GameObject path of a MonoBehaviour, like "MedBay/LevelController2/Level 3"."""
        go = o.read(check_read=False).m_GameObject.deref()
        names = []
        t = None
        for c in go.read().m_Component:
            obj = c.component.deref() if hasattr(c, "component") else c.deref()
            if obj is not None and obj.type.name == "Transform":
                t = obj.read()
                break
        while t is not None:
            names.append(t.m_GameObject.deref().read().m_Name)
            t = t.m_Father.deref().read() if t.m_Father.m_PathID else None
        return "/".join(reversed(names))


def load_localization(game):
    go = next(o for o in game.env.objects if o.type.name == "GameObject"
              and o.read().m_Name == "I2Languages" and o.assets_file.name == "resources.assets")
    mb = None
    for c in go.read().m_Component:
        obj = c.component.deref() if hasattr(c, "component") else c.deref()
        if obj.type.name == "MonoBehaviour":
            mb = obj
    r = Reader(mb.get_raw_data())
    spans = {}

    def s():
        n = r.un("i")
        v = r.b[r.p:r.p + n].decode("utf-8")
        spans[id(v)] = (mb.byte_start + r.p, n)
        r.p += n
        r.align()
        return v

    r.un("i"); r.un("q"); r.un("B"); r.align(); r.un("i"); r.un("q"); s()
    for _ in range(4):
        s()
    r.un("i")
    terms = {}
    for _ in range(r.un("i")):
        term = s(); r.un("i"); s()
        langs = [s() for _ in range(r.un("i"))]
        touch = [s() for _ in range(r.un("i"))]
        for _ in range(r.un("i")):
            r.un("i")
        terms[term] = (langs, touch)
    names = [s() for _ in range(r.un("i") * 2)][0::2]
    en = names.index("English")
    ru = next(i for i, n in enumerate(names) if n.startswith("Russian"))

    def get(key):
        if key not in terms:
            return None
        langs, touch = terms[key]

        def pick(i):
            v = langs[i] if i < len(langs) and langs[i] else (touch[i] if i < len(touch) else "")
            return v.strip()
        e, rr = pick(en), pick(ru)
        if not e:
            return None
        return {"en": e, "ru": rr or e, "ruOfficial": bool(rr)}

    # Where the English and Russian strings of a term lie in the game file.
    def ref(key):
        if key not in terms:
            return None
        langs, touch = terms[key]

        def raw(i):
            for v in (langs[i] if i < len(langs) else "", touch[i] if i < len(touch) else ""):
                if v.strip():
                    return v
            return None
        e, rr = raw(en), raw(ru)
        if e is None:
            return None
        rr = rr if rr is not None else e
        return [*spans[id(e)], *spans[id(rr)]], (e + rr).encode("utf-8")
    get.ref = ref
    get.node = mb.assets_file.name
    return get


RARITY = {1: "Common", 2: "Normal", 3: "Rare", 4: "Legendary"}
RESOURCES = {"m_food": "Food", "m_power": "Energy", "m_water": "Water", "m_stimPack": "StimPack",
             "m_radAway": "RadAway", "m_nuka": "Nuka"}
SPECIAL = ["Strength", "Perception", "Endurance", "Charisma", "Intelligence", "Agility", "Luck"]


def names(loc, key, fallback):
    n = loc(key) if key else None
    if n is None:
        return {"en": fallback, "ru": fallback, "ruOfficial": False, "locKey": None}
    n["locKey"] = key
    return n


def build(game_dir):
    g = Game(game_dir)
    loc = load_localization(g)

    def desc(key):
        n = loc(key) if key else None
        if n is not None:
            n["locKey"] = key
        return n
    sprites = {"Weapon": {}, "Outfit": {}, "Junk": {}, "Pet": {}, "Theme": {}}
    gp = g.read(g.of_class("GameParameters")[0])
    items = gp["m_items"]
    crafting = Crafting(items)
    junk_names = {j["m_JunkId"]: (loc(j["m_NameLocalizationId"]) or {}).get("en", j["m_JunkId"])
                  for j in items["m_junksList"]}

    def craft(kind, item):
        r = crafting.recipe(kind, item)
        if r is not None:
            r["junk"].sort(key=lambda e: (-e[1], junk_names.get(e[0], e[0])))
        return r
    cat = {
        "catalogVersion": CATALOG_VERSION,
        "gameVersion": g.game_version,
        "gameBuild": g.version,
    }

    weapons = []
    for w in items["m_weaponsList"] + items["m_ultraciteWeapons"]:
        weapons.append({
            "id": w["m_WeaponId"], "code": w["m_codeId"], "recipe": bool(w["m_canBeRecipe"]),
            "rarity": RARITY.get(w["m_itemRarity"], "Normal"),
            "damageMin": w["m_DamageMin"], "damageMax": w["m_DamageMax"], "weaponType": w["m_weaponType"],
            "hidden": bool(w["m_isHiddenItem"]), "name": names(loc, w["m_NameLocalizationId"], w["m_WeaponId"]),
            "desc": desc(w["m_descriptionLocalization"]), "sell": w["m_sellPrice"],
        })
        if w in items["m_weaponsList"] and (r := craft("Weapon", w)):
            weapons[-1]["craft"] = r
        sprites["Weapon"][w["m_WeaponId"]] = w["m_WeaponSprite"]
    cat["weapons"] = sorted(weapons, key=lambda x: x["id"].lower())

    outfits = []
    for o in items["m_outfitList"]:
        st = o["m_specialStats"]
        outfits.append({
            "id": o["m_outfitId"], "code": o["m_codeId"], "recipe": bool(o["m_canBeRecipe"]),
            "rarity": RARITY.get(o["m_itemRarity"], "Normal"),
            "special": [st[s]["Value"] for s in SPECIAL], "category": o["m_category"],
            "hidden": bool(o["m_isHiddenItem"]), "name": names(loc, o["m_outfitNameLocalizationId"], o["m_outfitId"]),
            "desc": desc(o["m_descriptionLocalization"]), "sell": o["m_sellPrice"],
        })
        if r := craft("Outfit", o):
            outfits[-1]["craft"] = r
        sprites["Outfit"][o["m_outfitId"]] = o["m_OutfitSprite"]
    cat["outfits"] = sorted(outfits, key=lambda x: x["id"].lower())

    cat["junk"] = sorted(({
        "id": j["m_JunkId"], "code": j["m_codeId"], "rarity": RARITY.get(j["m_itemRarity"], "Normal"),
        "name": names(loc, j["m_NameLocalizationId"], j["m_JunkId"]), "desc": desc(j["m_descriptionLocalization"]),
        "sell": j["m_sellPrice"],
    } for j in items["m_junksList"]), key=lambda x: x["id"].lower())
    for j in items["m_junksList"]:
        sprites["Junk"][j["m_JunkId"]] = j["m_JunkSprite"]

    room_types = load_enum(game_dir, "ERoomType")
    special_themes = load_enum(game_dir, "ESpecialTheme")
    cat["themes"] = sorted(({
        "id": t["m_codeId"], "roomType": t["m_roomType"], "themeType": t["m_themeType"], "recipe": bool(t["m_canBeRecipe"]),
        "room": room_types.get(t["m_roomType"], ""), "theme": special_themes.get(t["m_themeType"], ""),
        "name": names(loc, t["m_descriptionLocalization"], t["m_codeId"]),
        **({"craft": r} if (r := craft("Theme", t)) else {}),
    } for t in items["m_themesList"]), key=lambda x: x["id"].lower())
    for t in items["m_themesList"]:
        sprites["Theme"][t["m_codeId"]] = t["m_sprite"]

    event_keys = {"Xmas": "Christmas", "Halloween": "Halloween", "ThanksGiving": "Thanksgiving"}
    sched = {special_themes.get(x["m_themeType"]): x for x in gp["OverallThemeScheduledData"]}

    def date(d):
        return [d["m_month"], d["m_day"]]
    cat["eventThemes"] = [
        {"id": f"{room}_{ev}", "name": names(loc, f"Theme_Name_{loc_ev}_{'Diner' if room == 'Cafeteria' else room}", f"{room}_{ev}"),
         "start": date(sched[ev]["m_startDate"]), "end": date(sched[ev]["m_endDate"])}
        for room in ("Cafeteria", "LivingQuarters") for ev, loc_ev in event_keys.items()
    ]

    cat["decorations"] = sorted(({
        "id": d["m_DecorationId"], "name": names(loc, d["m_NameLocalizationId"], d["m_DecorationId"]),
    } for d in items["m_decorationParameters"]["m_decorationList"]), key=lambda x: x["id"].lower())

    pets_src = g.read(g.of_class("PetsCustomizationData")[0])
    breed_enum = load_enum(game_dir, "EPetBreed")
    bonus_enum = load_enum(game_dir, "EBonusEffect")
    pets = []
    bonuses = {}
    for index, p in enumerate(pets_src["m_petDataList"]):
        it = p["m_Item"]
        breed = breed_enum.get(it["m_breed"], str(it["m_breed"]))
        eff = it["m_bonusEffectList"][0] if it["m_bonusEffectList"] else None
        bonus = bonus_enum.get(eff["m_bonusEffect"], str(eff["m_bonusEffect"])) if eff else "None"
        bonuses.setdefault(bonus, names(loc, "Bonus_" + bonus, bonus))
        pets.append({
            "id": p["m_petID"], "code": str(index), "breed": breed, "breedCode": it["m_breed"], "petType": it["m_type"],
            "rarity": RARITY.get(it["m_itemRarity"], "Normal"), "bonus": bonus,
            "bonusMin": eff["m_minValue"] if eff else 0, "bonusMax": eff["m_maxValue"] if eff else 0,
            "name": names(loc, "Pet_" + breed, p["m_baseName"]), "desc": desc(it["m_descriptionLocalization"]),
        })
        sprites["Pet"][p["m_petID"]] = it["m_Sprite"]
    cat["pets"] = sorted(pets, key=lambda x: x["id"].lower())
    cat["petBonuses"] = dict(sorted(bonuses.items()))

    room_classes = load_enum(game_dir, "ERoomClass")
    build_list = g.read(g.of_class("UIRoomBuildList", prefer_shared=False)[0])
    unlock_of = {room_types.get(u["RoomType"]): u["Objective"]["Value"] for u in build_list["m_unlockRoomObjectives"]}
    reserve = {}
    for o in g.of_class("ProductionLevel", prefer_shared=False):
        parts = g.path(o).split("/")
        if len(parts) != 3 or not parts[1].startswith("LevelController") or not parts[2].startswith("Level "):
            continue
        res = g.read(o)["m_resourcesReserve"]
        merge, level = int(parts[1][len("LevelController"):]), int(parts[2][len("Level "):])
        reserve.setdefault(parts[0], {})[(merge, level)] = {
            RESOURCES[k]: round(v, 2) for k, v in res.items() if k in RESOURCES and v > 0}
    controllers = {}
    for o in g.of_class("LevelController", prefer_shared=False):
        try:
            parts = g.path(o).split("/")
        except Exception:
            continue
        if len(parts) == 2 and parts[1].startswith("LevelController"):
            controllers.setdefault(parts[0], {})[int(parts[1][len("LevelController"):])] = len(
                g.read(o)["m_roomLevels"])
    rooms = {}
    for o in g.of_class("RoomInfo", prefer_shared=False):
        d = g.read(o)
        t = room_types.get(d["m_eRoomType"])
        if not t or d["m_eRoomType"] >= 100 or t in rooms:
            continue
        ids = d["m_roomNamePerLevelLocId"]
        rooms[t] = {
            "type": t, "class": room_classes.get(d["m_roomClass"], ""), "maxMerge": d["m_maxMergeLevel"],
            "levels": len(ids) if ids else 1,
            "name": [names(loc, k, t) for k in ids] if ids else [names(loc, None, t)],
            "health": d["m_normalModeHealth"],
        }
        top = d["m_maxMergeLevel"]
        low = max(1, min(d["m_baseSize"]["m_col"] // 3, top))
        lc = controllers.get(t)
        per_merge = [(lc.get(m, 0) if lc else rooms[t]["levels"]) if low <= m <= top else 0 for m in range(1, 4)]
        rooms[t]["minMerge"] = low
        rooms[t]["levelsByMerge"] = per_merge
        rooms[t]["levels"] = max(per_merge) or rooms[t]["levels"]
        if t in unlock_of:
            rooms[t]["unlock"] = unlock_of[t]
        if t in reserve:
            rr = reserve[t]
            rooms[t]["reserve"] = [[rr.get((m, lv), {}) for lv in range(1, 4)] for m in range(1, 4)]
    cat["rooms"] = sorted(rooms.values(), key=lambda x: x["type"])

    dm = g.of_class("DwellerManager", prefer_shared=False)[0]
    dmd = g.read(dm)
    legendary = {g.read(x)["m_Name"] for x in (g.deref(dm, p) for p in dmd["m_legendaryDwellers"]) if x}
    rare = {g.read(x)["m_Name"] for x in (g.deref(dm, p) for p in dmd["m_rareDwellers"]) if x}

    def color(c):
        a, r, gg, b = (int(round(max(0.0, min(1.0, c[k])) * 255)) for k in "argb")
        return (a << 24) | (r << 16) | (gg << 8) | b

    def piece(owner, pptr):
        if not pptr or pptr["m_PathID"] == 0 or pptr["m_FileID"] != 0:
            return None
        x = g.deref(owner, pptr)
        return g.read(x)["m_Name"] if x else None

    uniq = []
    for o in g.of_class("UniqueDwellerData"):
        d = g.read(o)
        st = d["m_stats"]
        u = {"id": d["m_Name"], "firstName": d["m_name"], "lastName": d["m_lastName"],
             "gender": 2 if d["m_gender"] == 1 else 1, "outfit": d["m_outfitItemId"], "weapon": d["m_weaponItemId"],
             "rarity": "Legendary" if d["m_Name"] in legendary else "Rare" if d["m_Name"] in rare else "Common",
             "hidden": bool(d["m_isHiddenDweller"]), "randomName": bool(d["m_randomName"]),
             "randomBody": bool(d["m_randomBody"]),
             "special": [int(st["m_" + s.lower()]) for s in SPECIAL]}
        if not d["m_randomBody"]:
            u.update({"hair": piece(o, d["m_hairPiece"]), "faceMask": piece(o, d["m_facemask"]),
                      "skinColor": color(d["m_skinColor"]), "hairColor": color(d["m_hairColor"])})
        uniq.append(u)
    cat["uniqueDwellers"] = sorted(uniq, key=lambda x: x["id"].lower())

    quests = {}
    for o in g.of_class("QuestBankData"):
        for q in g.read(o)["m_questInformations"]:
            quests.setdefault(q["m_questName"], {
                "id": q["m_questName"], "name": names(loc, q["m_questTitleLocID"], q["m_questName"]),
            })
    cat["quests"] = sorted(quests.values(), key=lambda x: x["id"].lower())

    emergencies = load_enum(game_dir, "ERoomEmergencyMode")
    enemies = load_enum(game_dir, "EEnemyType")
    stats = load_enum(game_dir, "ESpecialStat")
    rewards = load_enum(game_dir, "EReward")

    def requirement(t):
        """Goal formula and description parameters of the first requirement of an objective."""
        for c in g.components(t) + g.child_components(t):
            cls = g.cls.get((c.assets_file.name, c.path_id), "")
            if not cls.startswith("Requirement"):
                continue
            d = g.read(c)
            inc = d.get("m_requirementIncreasePerLevel", 0)
            extra = []
            if cls == "RequirementEquip":
                parts = [v for v in (d["m_baseOutfits"], d["m_baseWeapons"]) if v > 0]
                base, step = sum(parts), inc * len(parts)
                first = parts[0] if parts else 0
            elif cls in ("RequirementDeltaResources", "RequirementCollectResourcesWithinTime"):
                res = d["m_baseGoalResources"]
                first = base = next((abs(v) for v in res.values() if v > 0), 0)
                step = inc
            else:
                field = next((k for k in d if k.startswith("m_base") and isinstance(d[k], (int, float))
                              and not isinstance(d[k], bool)), None)
                if field is None:
                    field = next((k for k in ("m_numberItemsToCollect", "m_targetCount") if k in d), None)
                first = base = d[field] if field else 0
                step = inc
            if cls == "RequirementStopIncident":
                mode = emergencies.get(d["m_incident"], "None")
                text = loc(mode + "_Name")
                if text:
                    text["locKey"] = mode + "_Name"
                extra.append({"text": text})
            elif cls == "RequirementKillEnemy":
                enemy = None if d["m_isRandom"] else enemies.get(d["m_enemyTypeTarget"])
                extra.append({"enemy": enemy})
            elif cls == "RequirementRaiseSpecialLevel":
                stat = None if d["m_anySpecialStat"] else stats.get(d["m_specificSpecialStat"])
                extra.append({"stat": None if stat == "None" else stat})
            elif cls == "RequirementEquipRandomWeapon":
                extra.append({"weapon": True})
            mx = d.get("m_requirementMaxValue", 0)
            max_level = max(0, int((mx - first) // inc)) if inc else None
            return {"class": cls, "requirementId": d.get("m_requirementID", ""), "base": int(base),
                    "step": int(step), "maxLevel": max_level, "extra": extra}
        return None

    def achievement_goal(t):
        """Target count of an achievement requirement (0 if it has none)."""
        for c in g.components(t) + g.child_components(t):
            cls = g.cls.get((c.assets_file.name, c.path_id), "")
            if "Requirement" not in cls and not cls.startswith("Achievement"):
                continue
            d = g.read(c)
            for k, v in d.items():
                low = k.lower()
                if (low.startswith(("m_base", "m_target", "m_achievementtarget")) or k == "m_mergeLevel")                         and isinstance(v, (int, float)) and not isinstance(v, bool) and v > 0:
                    return int(v)
            return 0
        return 0

    def objective_list(owner_cls, detailed):
        owner = g.of_class(owner_cls)[0]
        out = {}
        for p in g.read(owner)["m_objectives"]:
            t = g.deref(owner, p)
            if t is None:
                continue
            d = g.read(t)
            if d["m_objectiveID"] in out:
                continue
            key = d["m_descriptionLocalizerKey"]
            o = {"id": d["m_objectiveID"], "name": names(loc, key, d["m_objectiveID"])}
            if not detailed:
                req = requirement(t)
                o["goal"] = req["base"] if req else 0
                o["season"] = bool(d["m_isSeasonVaultExclusive"])
                o["room"] = next((r for r, u in unlock_of.items() if u == d["m_objectiveID"]), None)
            else:
                plural = loc(key + "_Plural") if key else None
                if plural:
                    plural["locKey"] = key + "_Plural"
                o.update({
                    "plural": plural, "level": d["m_level"], "tutorial": bool(d["m_isTutorial"]),
                    "normal": bool(d["m_isNormalMode"]), "survival": bool(d["m_isSurvivalMode"]),
                    "season": bool(d["m_isSeasonVaultExclusive"]),
                    "reward": rewards.get(d["m_baseRewardType"], str(d["m_baseRewardType"])),
                    "rewardAmount": d["m_baseRewardAmount"], "rewardStep": d["m_rewardIncrement"],
                    "goal": requirement(t),
                })
            out[d["m_objectiveID"]] = o
        return sorted(out.values(), key=lambda x: x["id"].lower())
    cat["objectives"] = objective_list("ObjectiveMgr", True)
    cat["unlockables"] = objective_list("UnlockablesMgr", False)

    ach_owner = g.of_class("AchievementsManager", prefer_shared=False)[0]
    ach = []
    for a in g.read(ach_owner)["m_achievementItems"]:
        t = g.deref(ach_owner, a["m_objective"])
        od = g.read(t) if t else {}
        key = od.get("m_descriptionLocalizerKey")
        ach.append({"id": a["m_id_iOS"], "objectiveId": od.get("m_objectiveID", ""), "steamId": a["m_steam_id"],
                    "name": names(loc, key, a["m_steam_id"] or a["m_id_iOS"]),
                    "goal": achievement_goal(t) if t else 0})
    cat["achievements"] = ach

    lt = g.read(g.of_class("LevelTables")[0])
    cat["levelExperience"] = [k["m_minimumExp"] for k in lt["m_character"]["m_keyLevels"]]

    dw = gp["m_dweller"]
    vault = g.read(g.of_class("Vault")[0])
    cat["limits"] = {
        "dwellerLevelMax": len(cat["levelExperience"]),
        "specialMin": 1, "specialMax": 10,
        "happinessMin": 0, "happinessMax": dw["m_happinessMaxValue"],
        "healthBase": dw["m_healthMaxValueBase"],
        "healthLevelBase": dw["m_healthLevelModifierBase"],
        "healthEnduranceFactor": dw["m_healthLevelModifierFactor"],
        "healthInitialEnduranceFactor": dw["m_healthEnduranceModifierFactor"],
        "roomLevelMax": 3, "mergeLevelMax": 3, "vaultThemeMax": 3, "lunchboxTypeMax": 3,
        "gridRows": vault["m_NumberOfRows"], "gridCols": vault["m_CellsPerRow"],
        "maxPets": vault["m_maximumPetCount"], "baseInventory": vault["m_baseMaxItems"],
        "serializeIdMax": 32767,
        "roomHealthPerLevel": gp["m_roomParameters"]["m_healthMultiplierPerLevel"],
        "roomHealthPerMerge": gp["m_roomParameters"]["m_healthMultiplierPerMerge"],
        "roomDamageMin": round(gp["m_emergencyParameters"]["m_minRoomDamagePercent"], 4),
        "roomDamageMax": round(gp["m_emergencyParameters"]["m_maxRoomDamagePercent"], 4),
        "teamItemsMax": dw["m_dwellerInventoryMaxItems"],
        "mrHandyHealth": dw["m_mrHandyHealth"],
    }

    def ui(keys):
        return {k: names(loc, v, k) for k, v in keys.items()}
    cat["names"] = {
        "special": ui({s: "Special_" + s for s in SPECIAL}),
        "rarity": ui({"Common": "Rarity_Common_Title", "Normal": "Rarity_Normal_Title",
                      "Rare": "Rarity_Rare_Title", "Legendary": "Rarity_Legendary_Title"}),
        "resources": ui({"Nuka": "Resource_Nuka", "Food": "Resource_Food", "Energy": "Resource_Energy",
                         "Water": "Resource_Water", "StimPack": "Resource_Stimpack", "RadAway": "Resource_Radaway"}),
        "enemies": {e: {"one": names(loc, e + "_Name", e), "many": names(loc, e + "_Name_Plural", e)}
                    for e in enemies.values() if e != "None"},
    }
    cat["art"] = build_art(g, gp, sprites, room_types, special_themes)
    cat["looks"] = cat["art"].pop("looks")

    # Phrases of the wasteland team journal, by the short id the save stores.
    wm = g.read(g.of_class("WastelandMessageManager", prefer_shared=False)[0])
    # Only positions are kept: the phrases are read from the player's game.
    crc = 0
    texts = {}
    for group, pairs in (("texts", sorted((e["Key"], e["Value"]) for e in wm["m_textList"])),
                         ("keys", sorted((k, k) for k in wm["m_aditionalKeys"]))):
        texts[group] = {}
        for key, term in pairs:
            found = loc.ref(term)
            if found is None or key in texts[group]:
                continue
            texts[group][key] = found[0]
            crc = zlib.crc32(found[1], crc)
    cat["wastelandTexts"] = {"node": loc.node, "crc": crc, "texts": texts["texts"], "keys": texts["keys"]}
    return cat


def load_enum(game_dir, name):
    """Reads enum members from Assembly-CSharp metadata via the decompiled source cache if present."""
    src = ENUM_SOURCES.get(name)
    if src is None:
        raise KeyError(name)
    return src


ENUM_SOURCES = {}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--game", required=True)
    ap.add_argument("--enums", required=True, help="JSON file with enum tables (see README.md)")
    ap.add_argument("--out", required=True)
    a = ap.parse_args()
    enums = json.loads(Path(a.enums).read_text(encoding="utf-8"))
    for k, v in enums.items():
        ENUM_SOURCES[k] = {int(n): m for n, m in v.items()}
    cat = build(a.game)
    Path(a.out).write_text(json.dumps(cat, ensure_ascii=False, separators=(",", ":")) + "\n", encoding="utf-8",
                           newline="\n")
    print("weapons", len(cat["weapons"]), "outfits", len(cat["outfits"]), "junk", len(cat["junk"]),
          "pets", len(cat["pets"]), "rooms", len(cat["rooms"]), "quests", len(cat["quests"]),
          "objectives", len(cat["objectives"]), file=sys.stderr)


if __name__ == "__main__":
    main()
