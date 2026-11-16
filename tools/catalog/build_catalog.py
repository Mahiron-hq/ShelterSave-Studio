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
from pathlib import Path

import UnityPy
from UnityPy.helpers.TypeTreeGenerator import TypeTreeGenerator

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

    def deref(self, owner, pptr):
        return self.by_id.get((owner.assets_file.name, pptr["m_PathID"]))


def load_localization(game):
    go = next(o for o in game.env.objects if o.type.name == "GameObject"
              and o.read().m_Name == "I2Languages" and o.assets_file.name == "resources.assets")
    mb = None
    for c in go.read().m_Component:
        obj = c.component.deref() if hasattr(c, "component") else c.deref()
        if obj.type.name == "MonoBehaviour":
            mb = obj
    r = Reader(mb.get_raw_data())

    def s():
        n = r.un("i")
        v = r.b[r.p:r.p + n].decode("utf-8")
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
    return get


RARITY = {1: "Common", 2: "Normal", 3: "Rare", 4: "Legendary"}
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
    gp = g.read(g.of_class("GameParameters")[0])
    items = gp["m_items"]
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
        })
    cat["weapons"] = sorted(weapons, key=lambda x: x["id"].lower())

    outfits = []
    for o in items["m_outfitList"]:
        st = o["m_specialStats"]
        outfits.append({
            "id": o["m_outfitId"], "code": o["m_codeId"], "recipe": bool(o["m_canBeRecipe"]),
            "rarity": RARITY.get(o["m_itemRarity"], "Normal"),
            "special": [st[s]["Value"] for s in SPECIAL], "category": o["m_category"],
            "hidden": bool(o["m_isHiddenItem"]), "name": names(loc, o["m_outfitNameLocalizationId"], o["m_outfitId"]),
        })
    cat["outfits"] = sorted(outfits, key=lambda x: x["id"].lower())

    cat["junk"] = sorted(({
        "id": j["m_JunkId"], "code": j["m_codeId"], "rarity": RARITY.get(j["m_itemRarity"], "Normal"),
        "name": names(loc, j["m_NameLocalizationId"], j["m_JunkId"]),
    } for j in items["m_junksList"]), key=lambda x: x["id"].lower())

    cat["themes"] = sorted(({
        "id": t["m_codeId"], "roomType": t["m_roomType"], "themeType": t["m_themeType"], "recipe": bool(t["m_canBeRecipe"]),
        "name": names(loc, t["m_descriptionLocalization"], t["m_codeId"]),
    } for t in items["m_themesList"]), key=lambda x: x["id"].lower())

    event_keys = {"Xmas": "Christmas", "Halloween": "Halloween", "ThanksGiving": "Thanksgiving"}
    cat["eventThemes"] = [
        {"id": f"{room}_{ev}", "name": names(loc, f"Theme_Name_{loc_ev}_{'Diner' if room == 'Cafeteria' else room}", f"{room}_{ev}")}
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
            "name": names(loc, "Pet_" + breed, p["m_baseName"]),
        })
    cat["pets"] = sorted(pets, key=lambda x: x["id"].lower())
    cat["petBonuses"] = dict(sorted(bonuses.items()))

    room_types = load_enum(game_dir, "ERoomType")
    room_classes = load_enum(game_dir, "ERoomClass")
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
        }
    cat["rooms"] = sorted(rooms.values(), key=lambda x: x["type"])

    uniq = []
    for o in g.of_class("UniqueDwellerData"):
        d = g.read(o)
        uniq.append({"id": d["m_Name"], "firstName": d["m_name"], "lastName": d["m_lastName"],
                     "gender": d["m_gender"], "outfit": d["m_outfitItemId"], "weapon": d["m_weaponItemId"]})
    cat["uniqueDwellers"] = sorted(uniq, key=lambda x: x["id"].lower())

    quests = {}
    for o in g.of_class("QuestBankData"):
        for q in g.read(o)["m_questInformations"]:
            quests.setdefault(q["m_questName"], {
                "id": q["m_questName"], "name": names(loc, q["m_questTitleLocID"], q["m_questName"]),
            })
    cat["quests"] = sorted(quests.values(), key=lambda x: x["id"].lower())

    def objective_list(owner_cls):
        owner = g.of_class(owner_cls)[0]
        out = {}
        for p in g.read(owner)["m_objectives"]:
            t = g.deref(owner, p)
            if t is None:
                continue
            d = g.read(t)
            out.setdefault(d["m_objectiveID"], {
                "id": d["m_objectiveID"], "name": names(loc, d["m_descriptionLocalizerKey"], d["m_objectiveID"]),
            })
        return sorted(out.values(), key=lambda x: x["id"].lower())
    cat["objectives"] = objective_list("ObjectiveMgr")
    cat["unlockables"] = objective_list("UnlockablesMgr")

    ach_owner = g.of_class("AchievementsManager", prefer_shared=False)[0]
    ach = []
    for a in g.read(ach_owner)["m_achievementItems"]:
        t = g.deref(ach_owner, a["m_objective"])
        key = g.read(t)["m_descriptionLocalizerKey"] if t else None
        ach.append({"id": a["m_id_iOS"], "steamId": a["m_steam_id"], "name": names(loc, key, a["m_steam_id"] or a["m_id_iOS"])})
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
    }

    def ui(keys):
        return {k: names(loc, v, k) for k, v in keys.items()}
    cat["names"] = {
        "special": ui({s: "Special_" + s for s in SPECIAL}),
        "rarity": ui({"Common": "Rarity_Common_Title", "Normal": "Rarity_Normal_Title",
                      "Rare": "Rarity_Rare_Title", "Legendary": "Rarity_Legendary_Title"}),
        "resources": ui({"Nuka": "Resource_Nuka", "Food": "Resource_Food", "Energy": "Resource_Energy",
                         "Water": "Resource_Water", "StimPack": "Resource_Stimpack", "RadAway": "Resource_Radaway"}),
    }
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
    Path(a.out).write_text(json.dumps(cat, ensure_ascii=False, separators=(",", ":")) + "\n", encoding="utf-8")
    print("weapons", len(cat["weapons"]), "outfits", len(cat["outfits"]), "junk", len(cat["junk"]),
          "pets", len(cat["pets"]), "rooms", len(cat["rooms"]), "quests", len(cat["quests"]),
          "objectives", len(cat["objectives"]), file=sys.stderr)


if __name__ == "__main__":
    main()
