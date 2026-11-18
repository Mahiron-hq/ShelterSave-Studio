# ShelterSave-Studio
# Copyright (C) 2026 Lev Burmistrov (Mahiron-hq)
# SPDX-License-Identifier: GPL-3.0-or-later
"""Locations of game pictures inside data.unity3d, for build_catalog.py.

Only positions, sizes, checksums and layout numbers are written. The editor
reads the pictures themselves from the player's installed game.
"""

import struct
import zlib

from UnityPy.helpers.MeshHelper import MeshHandler

FACE_TYPES = ["Smile", "Sick", "Dead", "Sad", "Angry", "Neutral", "Child", "Radiation", "Kiss", "Fear", "Child_Fear"]
ICON_ATLASES = ("Weapons_HD", "Outfits_HD", "Junks_HD", "Pet_Cats_HD", "Pet_Dogs_HD", "Pet_Macaws_HD",
                "Pet_FloatingDrone_HD", "Pet_Rollerbrain_HD")
FORMATS = {3: "RGB24", 4: "RGBA32", 5: "ARGB32", 10: "DXT1", 12: "DXT5"}
BYTES = {"RGB24": lambda w, h: w * h * 3, "RGBA32": lambda w, h: w * h * 4, "ARGB32": lambda w, h: w * h * 4,
         "DXT1": lambda w, h: (w + 3) // 4 * ((h + 3) // 4) * 8, "DXT5": lambda w, h: (w + 3) // 4 * ((h + 3) // 4) * 16}
ARMS = [[3, 4, 5], [6, 7, 8]]


class Art:
    def __init__(self, g):
        self.g = g
        bundle = list(g.env.files.values())[0]
        self.bundle = bundle
        self.files = {f.name: f for f in bundle.files.values() if hasattr(f, "objects")}
        self.textures = {}
        self.texture_ids = {}

    def resolve(self, owner, pptr):
        if not pptr or pptr["m_PathID"] == 0:
            return None
        f = owner.assets_file
        if pptr["m_FileID"] != 0:
            f = self.files.get(f.externals[pptr["m_FileID"] - 1].path.split("/")[-1])
        return f.objects.get(pptr["m_PathID"]) if f else None

    def texture(self, obj):
        d = obj.read()
        name = d.m_Name
        known = self.texture_ids.get(name)
        if known == (obj.assets_file.name, obj.path_id):
            return name
        if known is not None:
            name = f"{name}#{obj.path_id}"
            if name in self.textures:
                return name
        fmt = FORMATS.get(int(d.m_TextureFormat))
        if fmt is None:
            return None
        size = BYTES[fmt](d.m_Width, d.m_Height)
        if d.m_StreamData and d.m_StreamData.path:
            node, offset = d.m_StreamData.path.split("/")[-1], d.m_StreamData.offset
            res = self.bundle.files[node]
            res.Position = offset
            raw = res.read_bytes(size)
        else:
            raw = bytes(d.image_data)[:size]
            pos = obj.get_raw_data().find(raw)
            if pos < 0:
                return None
            node, offset = obj.assets_file.name, obj.byte_start + pos
        self.textures[name] = {
            "node": node, "offset": offset, "size": size, "format": fmt,
            "width": d.m_Width, "height": d.m_Height, "crc": zlib.crc32(raw),
        }
        self.texture_ids.setdefault(name, (obj.assets_file.name, obj.path_id))
        return name

    def material_texture(self, owner, pptr):
        m = self.resolve(owner, pptr)
        if m is None:
            return None
        for name, env in m.read().m_SavedProperties.m_TexEnvs:
            if name == "_MainTex":
                t = env.m_Texture
                f = m.assets_file if t.m_FileID == 0 else self.files.get(
                    m.assets_file.externals[t.m_FileID - 1].path.split("/")[-1])
                return f.objects.get(t.m_PathID) if f else None
        return None

    def icon_atlases(self):
        """Sprite rectangles of the HD item atlases, by sprite name."""
        out = {}
        for o in self.g.of_class("UIAtlas", prefer_shared=False):
            d = self.g.read(o)
            sprites = d.get("mSprites") or []
            if not sprites:
                continue
            t = self.material_texture(o, d["material"])
            if t is None or t.read().m_Name not in ICON_ATLASES:
                continue
            name = self.texture(t)
            if name is None:
                continue
            for s in sprites:
                out.setdefault(s["name"], [name, s["x"], s["y"], s["width"], s["height"]])
        return out

    def piece(self, owner, pptr):
        o = self.resolve(owner, pptr)
        if o is None:
            return None
        d = self.g.read(o)
        t = self.resolve(o, d["m_atlas"])
        tex = self.texture(t) if t is not None else None
        if tex is None:
            return None
        p = {"name": d["m_Name"], "tex": tex, "offset": [d["m_offset"]["x"], d["m_offset"]["y"]],
             "scale": [d["m_scale"]["x"], d["m_scale"]["y"]]}
        for k, out in (("m_useHairColor", "hairColor"), ("m_useSkinColor", "skinColor")):
            if d.get(k):
                p[out] = True
        if d.get("m_useCustomSkinColor"):
            c = d["m_customSkinColor"]
            p["customSkin"] = [round(c["r"], 4), round(c["g"], 4), round(c["b"], 4)]
        if "m_faceType" in d:
            p["faceType"] = d["m_faceType"]
        return p

    def plain(self, owner, pptr):
        """Atlas, offset and scale of a piece without extra flags."""
        o = self.resolve(owner, pptr)
        if o is None:
            return None, None
        d = self.g.read(o)
        t = self.resolve(o, d["m_atlas"])
        tex = self.texture(t) if t is not None else None
        if tex is None:
            return o, None
        return o, {"tex": tex, "offset": [d["m_offset"]["x"], d["m_offset"]["y"]],
                   "scale": [d["m_scale"]["x"], d["m_scale"]["y"]]}

    def outfit(self, owner, pptr):
        """Everything the dweller shader takes from an outfit."""
        o, base = self.plain(owner, pptr)
        if base is None:
            return None
        d = self.g.read(o)
        out = dict(base)
        colors = [color_argb(c) for c in d["m_colors"]]
        if colors:
            out["colors"] = colors
        if d["m_hasSkirt"]:
            out["skirt"] = True
        if d["m_largeHeadgear"]["m_PathID"]:
            out["largeHeadgear"] = True
        _, mask = self.plain(o, d["m_coloringMask"])
        if mask:
            out["coloringMask"] = mask
        poses = d["m_glovePoses"]
        go, glove = self.plain(o, poses[0]) if poses else (None, None)
        if go is not None:
            gd = self.g.read(go)
            out["glove"] = glove
            out["gloveHands"] = bool(gd["m_shouldRenderHands"])
        helmet = self.helmet(o, d["m_helmet"])
        if helmet:
            out["helmet"] = helmet
        return out

    def helmet(self, owner, pptr):
        ho, helmet = self.plain(owner, pptr)
        if not helmet:
            return None
        hd = self.g.read(ho)
        helmet.update({"exclusive": bool(hd["m_isExclusive"]), "noMask": bool(hd["m_isExcludingFaceMask"]),
                       "removeInVault": bool(hd["m_shouldRemoveInVault"])})
        _, hmask = self.plain(ho, hd["m_mask"])
        if hmask:
            helmet["mask"] = hmask
        return helmet

    def mesh(self, owner, pptr):
        o = self.resolve(owner, pptr)
        d = o.read()
        h = MeshHandler(d)
        h.process()
        raw = o.get_raw_data()
        vd = bytes(d.m_VertexData.m_DataSize)
        ib = bytes(d.m_IndexBuffer)
        n = d.m_VertexData.m_VertexCount
        ch = d.m_VertexData.m_Channels
        assert (ch[0].stream, ch[0].format, ch[0].dimension) == (0, 0, 3), "position layout"
        assert (ch[4].stream, ch[4].offset, ch[4].format, ch[4].dimension) == (1, 0, 0, 2), "uv layout"
        assert (ch[13].stream, ch[13].format, ch[13].dimension) == (2, 10, 1), "bone layout"
        s1 = (12 * n + 15) // 16 * 16
        s2 = (s1 + 16 * n + 15) // 16 * 16
        assert struct.unpack_from("<2f", vd, s1) == tuple(h.m_UV0[0]), "uv offset"
        vpos, ipos = raw.find(vd), raw.find(ib)
        assert vpos >= 0 and ipos >= 0 and int(d.m_IndexFormat) == 0
        origins = []
        for m in d.m_BindPose:
            t = (m.e03, m.e13, m.e23)
            origins.append([round(-(m.e00 * t[0] + m.e10 * t[1] + m.e20 * t[2]), 5),
                            round(-(m.e01 * t[0] + m.e11 * t[1] + m.e21 * t[2]), 5)])
        return {
            "node": o.assets_file.name, "vertexOffset": o.byte_start + vpos, "vertexSize": len(vd),
            "vertexCrc": zlib.crc32(vd), "vertexCount": n, "uvStart": s1, "boneStart": s2,
            "indexOffset": o.byte_start + ipos, "indexSize": len(ib), "indexCrc": zlib.crc32(ib),
            "armJoints": {str(b): origins[b] for chain in ARMS for b in chain},
        }

    def dweller(self, cat_obj, cat, gp_obj, jumpsuit, gender):
        lst_o = self.resolve(cat_obj, cat[f"m_{gender}List"])
        lst = self.g.read(lst_o)
        hands = [self.piece(lst_o, x) for x in lst["m_handPoses"]]
        out = {"body": self.piece(lst_o, lst["m_body"]), "hands": next(h for h in hands if h and h["name"] == "open")}
        out["faces"] = {FACE_TYPES[p["faceType"]]: p for p in (self.piece(lst_o, x) for x in lst["m_faces"]) if p}
        out["hairs"] = {p["name"]: p for p in (self.piece(lst_o, x) for x in lst["m_hairs"]) if p}
        out["faceMasks"] = {p["name"]: p for key in ("m_faceMasks", "m_beards")
                            for p in (self.piece(lst_o, x) for x in lst[key]) if p}

        def names(key):
            return [self.g.read(o)["m_Name"] for o in (self.resolve(lst_o, x) for x in lst[key]) if o is not None]
        out["looks"] = {"hairs": names("m_hairs"), "beards": names("m_beards"), "masks": names("m_faceMasks"),
                        "skinColors": [color_argb(c) for c in lst["m_skinColors"]],
                        "hairColors": [color_argb(c) for c in lst["m_hairColorsForCustomization"]]}
        oo = self.resolve(gp_obj, jumpsuit[f"m_{gender}Outfit"])
        od = self.g.read(oo)
        out["outfit"] = {"tex": self.texture(self.resolve(oo, od["m_atlas"])),
                         "offset": [od["m_offset"]["x"], od["m_offset"]["y"]],
                         "scale": [od["m_scale"]["x"], od["m_scale"]["y"]]}
        off = cat[f"m_{gender}Offsets"]
        out["handOffset"] = [off["handOffset"]["x"], off["handOffset"]["y"]]
        out["faceOffset"] = [off["faceOffset"]["x"], off["faceOffset"]["y"]]
        mat = self.resolve(cat_obj, cat[f"m_{gender}RealtimeMaterial"]).read()
        colors = dict(mat.m_SavedProperties.m_Colors)
        out["faceClip"] = [round(x, 4) for x in (colors["_FaceClipping"].r, colors["_FaceClipping"].g,
                                                 colors["_FaceClipping"].b, colors["_FaceClipping"].a)]
        out["handClip"] = [round(x, 4) for x in (colors["_HandClipping"].r, colors["_HandClipping"].g,
                                                 colors["_HandClipping"].b, colors["_HandClipping"].a)]
        out["mesh"] = self.mesh(cat_obj, cat[f"m_{gender}Meshes"]["m_adultMesh"])
        out["arms"] = ARMS
        skirt = self.piece(lst_o, lst["m_skirtBody"]) if "m_skirtBody" in lst else None
        if skirt:
            out["skirtBody"] = skirt
        outfits = {}
        for item in self.g_outfits:
            o = self.outfit(gp_obj, item[f"m_{gender}Outfit"])
            if o:
                outfits[item["m_outfitId"]] = o
        preg = self.outfit(cat_obj, cat["m_pregnantOutfit"]) if gender == "female" else None
        if preg:
            outfits["__pregnant"] = preg
        out["outfits"] = outfits
        helmets = {}
        for x in lst["m_helmets"]:
            ho = self.resolve(lst_o, x)
            h = self.helmet(lst_o, x)
            if h and ho is not None:
                helmets[self.g.read(ho)["m_Name"]] = h
        out["helmets"] = helmets
        # Outfits without their own colouring mask are tinted through this one.
        atl_o = self.resolve(cat_obj, cat["m_dwellerAtlases"])
        ref = self.g.read(self.resolve(atl_o, self.g.read(atl_o)[f"m_{gender}BurnAtlas"]))
        burn = self.named_texture(ref["m_atlasName"])
        out["burnMask"] = self.texture(burn) if burn is not None else None
        return out

    def named_texture(self, name):
        """Texture loaded by name from Resources; the HD copy is the uncompressed one."""
        found = [o for o in self.g.env.objects if o.type.name == "Texture2D" and o.read().m_Name == name]
        found.sort(key=lambda o: int(o.read().m_TextureFormat) != 4)
        return found[0] if found else None


def color_argb(c):
    a, r, g, b = (int(round(max(0.0, min(1.0, c[k])) * 255)) for k in "argb")
    return (a << 24) | (r << 16) | (g << 8) | b


def room_pictures(art, g, room_types, special_themes):
    """Pre-rendered room pictures (impostors) and vault exterior thumbnails."""
    out = {}
    for o in g.of_class("Catalog", prefer_shared=False):
        d = g.read(o)
        if "m_atlases" not in d:
            continue
        for a in d["m_atlases"]:
            t = art.resolve(o, a["m_tex"])
            tex = art.texture(t) if t is not None else None
            if tex is None:
                continue
            info = art.textures[tex]
            w, h = info["width"], info["height"]
            for i in a["m_impostors"]:
                kind = room_types.get(i["m_roomType"])
                if not kind:
                    continue
                x, y = i["m_offset"]["x"] * w, i["m_offset"]["y"] * h
                sw, sh = i["m_scale"]["x"] * w, i["m_scale"]["y"] * h
                out.setdefault(f"Room:{kind}:{i['m_roomLevel']}:{i['m_roomSize']}",
                               [tex, round(x), round(h - y - sh), round(sw), round(sh)])
    for o in g.of_class("SpecialThemeObjectSpawner", prefer_shared=False):
        d = g.read(o)
        for e in [d["DefaultAlternateObject"]] + d["AlternateObjects"] + d["AdditionalAlternateObjects"]:
            t = art.resolve(o, e["Thumbnail"])
            tex = art.texture(t) if t is not None else None
            theme = special_themes.get(e["Theme"])
            if tex is None or theme is None:
                continue
            info = art.textures[tex]
            out.setdefault(f"Exterior:{theme}", [tex, 0, 0, info["width"], info["height"]])
    return out


def build_art(g, gp, sprites, room_types, special_themes):
    """sprites: {"Weapon": {id: sprite}, ...}. Returns the catalog "art" section."""
    art = Art(g)
    art.g_outfits = gp["m_items"]["m_outfitList"]
    rects = art.icon_atlases()
    icons = {}
    for kind, table in sprites.items():
        for item_id, sprite in table.items():
            if sprite in rects:
                icons[f"{kind}:{item_id}"] = rects[sprite]
    icons.update(room_pictures(art, g, room_types, special_themes))
    cat_obj = next(o for o in g.of_class("Catalog", prefer_shared=False) if "m_maleList" in g.read(o))
    cat = g.read(cat_obj)
    gp_obj = g.of_class("GameParameters")[0]
    jumpsuit = next(o for o in gp["m_items"]["m_outfitList"] if o["m_outfitId"].lower() == "jumpsuit")
    dwellers = {gender: art.dweller(cat_obj, cat, gp_obj, jumpsuit, gender) for gender in ("male", "female")}
    looks = {gender: kit.pop("looks") for gender, kit in dwellers.items()}
    states = gp["m_dweller"]["m_happynessStateOrder"]
    face = {4: "Smile", 3: "Smile", 2: "Neutral", 1: "Sad", 0: "Sad"}
    return {
        "bundle": "data.unity3d",
        "textures": dict(sorted(art.textures.items())),
        "icons": dict(sorted(icons.items())),
        "dweller": dwellers,
        "faceByHappiness": [[s["minHappiness"], face[s["state"]]] for s in states],
        "looks": looks,
    }
