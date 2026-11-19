// ShelterSave-Studio
// Copyright (C) 2026 Lev Burmistrov (Mahiron-hq)
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! Readable texts built from save data, shared by both editions.

use crate::actions::ActionError;
use crate::catalog::{catalog, game_texts, ItemKind, Objective};
use crate::edit::Label;
use crate::i18n::{self, Lang};
use crate::integrity::Issue;
use crate::json::{path_to_string, Seg, Value};
use crate::rules::RuleError;
use crate::vault;

/// Interface language and the item name setting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tx {
    pub lang: Lang,
    /// Item, weapon and pet names in Russian.
    pub item_ru: bool,
}

impl Tx {
    pub fn new(lang: Lang, item_ru: bool) -> Tx {
        Tx { lang, item_ru }
    }

    pub fn t(&self, key: &str) -> String {
        i18n::tr(self.lang, key)
    }

    pub fn tf(&self, key: &str, params: &[(&str, String)]) -> String {
        i18n::trf(self.lang, key, params)
    }

    pub fn ru(&self) -> bool {
        self.lang == Lang::Ru
    }

    /// Journal entry text; `root` resolves paths to readable places.
    pub fn label_text(&self, label: &Label, root: Option<&Value>) -> String {
        let params: Vec<(&str, String)> = label
            .params
            .iter()
            .map(|(k, v)| (k.as_str(), self.param_text(k, v, root)))
            .collect();
        self.tf(&label.key, &params)
    }

    /// Readable form of an id or path stored in a journal entry.
    pub fn param_text(&self, key: &str, v: &str, root: Option<&Value>) -> String {
        let ru = self.ru();
        let cat = catalog();
        match key {
            "room" => cat
                .room(v)
                .and_then(|r| r.name.first())
                .map_or_else(|| v.to_string(), |n| n.get(ru).to_string()),
            "id" | "item" => ItemKind::ALL
                .iter()
                .find_map(|k| cat.item(*k, v))
                .map(|i| i.name.get(ru).to_string())
                .or_else(|| {
                    cat.quests
                        .iter()
                        .find(|q| q.id == v)
                        .map(|q| q.name.get(ru).to_string())
                })
                .or_else(|| {
                    cat.objective(v)
                        .map(|o| o.text(ru, 0, &["…".into(), "…".into()]))
                })
                .or_else(|| {
                    cat.achievements
                        .iter()
                        .find(|a| !v.is_empty() && (a.objective_id == v || a.id == v))
                        .map(|a| a.name.get(ru).to_string())
                })
                .unwrap_or_else(|| v.to_string()),
            "key" => {
                let k = format!("res.{v}");
                match self.t(&k) {
                    t if t == k => v.to_string(),
                    t => t,
                }
            }
            "path" | "field" => match (parse_path(v), root) {
                (Some(path), Some(root)) if !path.is_empty() => self.path_title(root, &path),
                _ => v.to_string(),
            },
            _ => v.to_string(),
        }
    }

    pub fn error_text(&self, e: &ActionError) -> String {
        match e {
            ActionError::Rule(RuleError::OutOfRange { min, max }) => self.tf(
                "aerr.outOfRange",
                &[("min", min.to_string()), ("max", max.to_string())],
            ),
            ActionError::Rule(RuleError::NotAllowed) => self.t("aerr.notAllowed"),
            ActionError::Rule(RuleError::NotInCatalog) => self.t("aerr.notInCatalog"),
            ActionError::Rule(RuleError::WrongType) => self.t("aerr.wrongType"),
            ActionError::Rule(RuleError::ExpertOnly) => self.t("aerr.expertOnly"),
            ActionError::Rule(RuleError::TooManyDwellers) => self.t("aerr.tooManyDwellers"),
            ActionError::Rule(RuleError::RoomOverlap) => self.t("aerr.roomOverlap"),
            ActionError::Rule(RuleError::RoomDisconnected) => self.t("aerr.roomDisconnected"),
            ActionError::Rule(RuleError::RoomBusy) => self.t("aerr.roomBusy"),
            ActionError::Rule(RuleError::RoomFixed) => self.t("aerr.roomFixed"),
            ActionError::Rule(RuleError::RoomUnique) => self.t("aerr.roomUnique"),
            ActionError::Rule(RuleError::RoomLocked) => self.t("aerr.roomLocked"),
            ActionError::Rule(RuleError::RoomSize) => self.t("aerr.roomSize"),
            ActionError::Rule(RuleError::OutfitGender) => self.t("aerr.outfitGender"),
            ActionError::Rule(RuleError::IncidentActive) => self.t("aerr.incidentActive"),
            ActionError::Rule(RuleError::InventoryFull) => self.t("aerr.inventoryFull"),
            ActionError::Rule(RuleError::NotFound) => self.t("aerr.notFound"),
            ActionError::BadJson(d) => self.tf("aerr.badJson", &[("detail", d.clone())]),
            ActionError::Nothing => self.t("aerr.nothing"),
            ActionError::Edit(d) => self.tf("aerr.edit", &[("detail", d.clone())]),
        }
    }

    pub fn issue_text(&self, i: &Issue) -> String {
        let params: Vec<(&str, String)> = i
            .params
            .iter()
            .map(|(k, v)| (k.as_str(), v.clone()))
            .collect();
        self.tf(&format!("integ.{}", i.code), &params)
    }

    /// Readable place of a value: the dweller or room it belongs to, then the
    /// field names, for change lists.
    pub fn path_title(&self, root: &Value, path: &[Seg]) -> String {
        let key = |i: usize| match path.get(i) {
            Some(Seg::Key(k)) => k.as_str(),
            _ => "",
        };
        let mut parts: Vec<String> = Vec::new();
        let mut start = 0;
        match (key(0), key(1), path.get(2)) {
            ("dwellers", "dwellers", Some(Seg::Index(i))) => {
                if let Some(d) = vault::arr(root, &vault::DWELLERS).get(*i) {
                    parts.push(vault::dweller_name(d));
                }
                start = 3;
            }
            ("vault", "rooms", Some(Seg::Index(i))) => {
                if let Some(r) = vault::arr(root, &vault::ROOMS).get(*i) {
                    parts.push(self.room_title(r));
                }
                start = 3;
            }
            _ => {}
        }
        let mut prefix: Vec<Seg> = path[..start.min(path.len())].to_vec();
        let mut fields = Vec::new();
        for seg in &path[start.min(path.len())..] {
            prefix.push(seg.clone());
            if let Seg::Key(_) = seg {
                fields.push(crate::fields::describe(self.lang, &prefix).label);
            }
        }
        if fields.len() > 3 {
            fields.drain(..fields.len() - 3);
        }
        parts.extend(fields);
        if parts.is_empty() {
            path_to_string(path)
        } else {
            parts.join(" › ")
        }
    }

    pub fn quest_name(&self, id: &str) -> String {
        catalog()
            .quests
            .iter()
            .find(|q| q.id == id)
            .map_or_else(|| id.to_string(), |q| q.name.get(self.ru()).to_string())
    }

    /// Quest name with its difficulty word, for lists.
    pub fn quest_line(&self, id: &str) -> String {
        match quest_difficulty(id) {
            Some(d) => {
                let key = format!("o.diffWord.{d}");
                let d = match self.t(&key) {
                    t if t == key => d,
                    t => t,
                };
                format!(
                    "{}  ·  {}",
                    self.quest_name(id),
                    self.tf("o.difficulty", &[("n", d)])
                )
            }
            None => self.quest_name(id),
        }
    }

    /// Dates the game writes as `M/d/yyyy h:mm:ss AM` or `MM/dd/yyyy HH:mm:ss`.
    pub fn game_date(&self, s: &str) -> String {
        let parse = || -> Option<(u32, u32, u32, u32, u32)> {
            let mut it = s.split_whitespace();
            let mut d = it.next()?.split('/').map(|x| x.parse::<u32>().ok());
            let (mo, day, y) = (d.next()??, d.next()??, d.next()??);
            let mut t = it.next()?.split(':').map(|x| x.parse::<u32>().ok());
            let (mut h, mi) = (t.next()??, t.next()??);
            match it.next() {
                Some("PM") if h < 12 => h += 12,
                Some("AM") if h == 12 => h = 0,
                _ => {}
            }
            Some((y, mo, day, h, mi))
        };
        match parse() {
            Some((y, mo, d, h, mi)) if self.ru() => format!("{d:02}.{mo:02}.{y} {h:02}:{mi:02}"),
            Some((y, mo, d, h, mi)) => format!("{y}-{mo:02}-{d:02} {h:02}:{mi:02}"),
            None => s.to_string(),
        }
    }

    /// "2 h 5 min" or "5 min".
    pub fn duration(&self, secs: f64) -> String {
        let total = secs.max(0.0) as i64;
        let (h, m) = (total / 3600, (total % 3600) / 60);
        if h > 0 {
            self.tf("o.hm", &[("h", h.to_string()), ("m", m.to_string())])
        } else {
            self.tf("o.m", &[("m", m.to_string())])
        }
    }

    /// One wasteland journal line as the game shows it: "HHMM@part@part", each
    /// part "text#param#param"; a leading letter says how to read the piece.
    pub fn log_line(&self, code: &str) -> (String, String) {
        let ru = self.ru();
        let items = self.item_ru;
        let mut parts = code.split('@').filter(|s| !s.is_empty());
        let time = parts
            .next()
            .filter(|t| t.len() > 2)
            .map(|t| format!("{}:{}", &t[..t.len() - 2], &t[t.len() - 2..]))
            .unwrap_or_default();
        let texts = game_texts();
        // A phrase from the game: Some(None) when the game was not read.
        let phrase = |s: &str| -> Option<Option<String>> {
            let (kind, rest) = (s.chars().next()?, s.get(1..).unwrap_or(""));
            let group = match kind {
                'l' => texts.as_ref().map(|t| &t.texts),
                'f' => texts.as_ref().map(|t| &t.keys),
                _ => return None,
            };
            Some(group.map(|g| {
                g.get(rest)
                    .map_or_else(|| rest.to_string(), |n| n.get(ru).to_string())
            }))
        };
        let piece = |s: &str| -> String {
            if let Some(p) = phrase(s) {
                return p.unwrap_or_default();
            }
            let (Some(kind), rest) = (s.chars().next(), s.get(1..).unwrap_or("")) else {
                return String::new();
            };
            match kind {
                'r' => rest
                    .split('&')
                    .filter(|x| !x.is_empty())
                    .map(|x| {
                        let (n, amount) = x.split_at(1);
                        let key = ["Nuka", "Food", "Energy", "Water", "StimPack", "RadAway"]
                            .get(n.parse::<usize>().unwrap_or(99))
                            .copied()
                            .unwrap_or("");
                        let name = self.t(&format!("res.{key}"));
                        format!("{amount} {name}")
                    })
                    .collect::<Vec<_>>()
                    .join(", "),
                'i' => rest
                    .split('&')
                    .filter(|x| !x.is_empty())
                    .map(|x| {
                        let (k, id) = x.split_at(1);
                        let by_code = |kind: ItemKind| {
                            catalog()
                                .items(kind)
                                .find(|it| item_code(kind, it.id).as_deref() == Some(id))
                                .map(|it| it.name.get(items).to_string())
                        };
                        match k {
                            "w" => by_code(ItemKind::Weapon),
                            "o" => by_code(ItemKind::Outfit),
                            "p" => by_code(ItemKind::Pet),
                            "j" => by_code(ItemKind::Junk),
                            "r" => Some(self.t("o.logRecipe")),
                            _ => None,
                        }
                        .unwrap_or_else(|| id.to_string())
                    })
                    .collect::<Vec<_>>()
                    .join(", "),
                _ => rest.to_string(),
            }
        };
        let text = parts
            .map(|part| {
                let mut bits = part.split('#').filter(|s| !s.is_empty());
                let head = bits.next().unwrap_or("");
                let args: Vec<String> = bits.map(piece).collect();
                if matches!(phrase(head), Some(None)) {
                    // Without the game only the values are known.
                    let note = self.t("o.logNoText");
                    let known: Vec<&str> = args
                        .iter()
                        .map(String::as_str)
                        .filter(|a| !a.is_empty())
                        .collect();
                    return if known.is_empty() {
                        note
                    } else {
                        format!("{note}: {}", known.join(", "))
                    };
                }
                let mut line = piece(head);
                for (n, arg) in args.iter().enumerate() {
                    line = line.replace(&format!("{{{n}}}"), arg);
                }
                line
            })
            .collect::<Vec<_>>()
            .join(" ");
        (time, text)
    }

    pub fn incident_name(&self, mode: i64) -> String {
        let key = format!("inc.mode.{mode}");
        match self.t(&key) {
            s if s == key => mode.to_string(),
            s => s,
        }
    }

    /// Values for `{1}` and later in an objective text; parts the game picks on
    /// load show a placeholder.
    pub fn objective_params(&self, o: &Objective, req: Option<&Value>, goal: i64) -> Vec<String> {
        let ru = self.ru();
        let later = self.t("o.gamePicks");
        let saved = |key: &str| {
            req.and_then(|r| r.get(key))
                .and_then(Value::as_text)
                .filter(|s| !s.is_empty() && s != "None")
        };
        let Some(g) = &o.goal else {
            return Vec::new();
        };
        g.extra
            .iter()
            .map(|e| {
                if let Some(t) = &e.text {
                    return t.get(ru).to_string();
                }
                if let Some(enemy) = &e.enemy {
                    let kind = enemy.clone().or_else(|| saved("enemyTypeTarget"));
                    return kind
                        .and_then(|k| {
                            catalog()
                                .names
                                .enemies
                                .get(&k)
                                .map(|n| (k.clone(), n.clone()))
                        })
                        .map_or(later.clone(), |(_, n)| {
                            if goal > 1 { n.many } else { n.one }.get(ru).to_string()
                        });
                }
                if let Some(stat) = &e.stat {
                    let s = stat.clone().or_else(|| saved("specificSpecialStat"));
                    return s
                        .and_then(|s| catalog().names.special.get(&s))
                        .map_or(later.clone(), |n| n.get(ru).to_string());
                }
                if e.weapon {
                    return req
                        .and_then(|r| r.get("targetWeaponIDs"))
                        .and_then(Value::as_array)
                        .and_then(|a| a.first())
                        .and_then(Value::as_text)
                        .and_then(|id| {
                            catalog()
                                .item(ItemKind::Weapon, &id)
                                .map(|i| i.name.get(ru).to_string())
                        })
                        .unwrap_or_else(|| later.clone());
                }
                String::new()
            })
            .collect()
    }

    pub fn objective_text(&self, id: &str, level: i64, req: Option<&Value>) -> String {
        let ru = self.ru();
        match catalog().objective(id) {
            Some(o) => {
                let params = self.objective_params(o, req, o.goal_value(level));
                o.text(ru, level, &params)
            }
            None => id.to_string(),
        }
    }

    pub fn reward_text(&self, o: &Objective, level: i64) -> String {
        self.tf(
            &format!("o.reward.{}", o.reward),
            &[("n", o.reward_value(level).to_string())],
        )
    }

    /// Names a room type has at its levels, without repeats.
    pub fn level_names(&self, kind: &str) -> Vec<String> {
        let levels = catalog().room(kind).map_or(1, |r| r.name.len().max(1)) as i64;
        let mut out: Vec<String> = Vec::new();
        for l in 1..=levels {
            let n = self.kind_name(kind, l);
            if n != kind && !out.contains(&n) {
                out.push(n);
            }
        }
        if out.is_empty() {
            out.push(self.kind_name(kind, 1));
        }
        out
    }

    pub fn kind_name(&self, kind: &str, level: i64) -> String {
        if kind == "FakeWasteland" {
            return self.t("map.wasteland");
        }
        let level = level.clamp(1, 3) as usize;
        match catalog().room(kind) {
            Some(info) => {
                let n = info.name.get(level - 1).or_else(|| info.name.first());
                n.map_or(kind.to_string(), |n| {
                    if self.ru() {
                        n.get(true).to_string()
                    } else {
                        n.en.clone()
                    }
                })
            }
            None => kind.to_string(),
        }
    }

    /// Room name with its floor and cell, for lists.
    pub fn room_title(&self, r: &Value) -> String {
        format!(
            "{}  ·  {}",
            self.room_name(r),
            self.tf(
                "room.position",
                &[
                    ("row", vault::int(r, &["row"]).unwrap_or(0).to_string()),
                    ("col", vault::int(r, &["col"]).unwrap_or(0).to_string()),
                ],
            )
        )
    }

    pub fn room_name(&self, r: &Value) -> String {
        let kind = vault::text(r, &["type"]).unwrap_or_default();
        self.kind_name(&kind, vault::int(r, &["level"]).unwrap_or(1))
    }
    pub fn hat_name(&self, key: &str) -> Option<String> {
        let k: String = key
            .to_lowercase()
            .replace("female", "")
            .replace("male", "")
            .chars()
            .filter(char::is_ascii_alphanumeric)
            .collect();
        HAT_NAMES
            .iter()
            .find(|(n, _, _)| *n == k)
            .map(|(_, ru, en)| if self.ru() { ru } else { en }.to_string())
    }

    /// Readable name of a hair or face mask from its game name.
    pub fn look_name(&self, name: &str) -> String {
        if let Ok(n) = name.parse::<u32>() {
            return self.tf("dw.variant", &[("n", n.to_string())]);
        }
        if let Some(n) = name
            .strip_prefix("f_hair_")
            .and_then(|n| n.parse::<u32>().ok())
        {
            return self.tf("dw.beard", &[("n", n.to_string())]);
        }
        if let Some((a, b)) = name
            .split_once('_')
            .filter(|(a, b)| a.parse::<u32>().is_ok() && b.parse::<u32>().is_ok())
        {
            return self.tf(
                "dw.variant",
                &[("n", format!("{}.{b}", a.trim_start_matches('0')))],
            );
        }
        let mut plain = name
            .trim_start_matches("hair_")
            .trim_start_matches("Hair_")
            .replace(['_', '-'], " ");
        if self.ru() {
            for (en, ru) in LOOK_WORDS.iter() {
                plain = plain.replace(en, ru);
            }
        }
        self.tf("dw.lookNamed", &[("name", plain.trim().to_string())])
    }
}

/// Difficulty number from ids such as `Daily_05_Diff_30_B`.
pub fn quest_difficulty(id: &str) -> Option<String> {
    let rest = id.split("_Diff_").nth(1)?;
    let n: String = rest
        .chars()
        .take_while(char::is_ascii_alphanumeric)
        .collect();
    (!n.is_empty()).then_some(n)
}

/// Collection code of an item (`m_codeId`, pet index or breed number).
pub fn item_code(kind: ItemKind, id: &str) -> Option<String> {
    let c = catalog();
    match kind {
        ItemKind::Weapon => c.weapon(id).map(|w| w.code.clone()),
        ItemKind::Outfit => c.outfit(id).map(|o| o.code.clone()),
        ItemKind::Pet => c.pet(id).map(|p| p.code.clone()),
        ItemKind::Junk => c
            .junk
            .iter()
            .find(|j| j.id == id)
            .and_then(|j| j.code.clone()),
    }
}

/// Reverse of `path_to_string`: `a.b[2].c`.
pub fn parse_path(s: &str) -> Option<Vec<Seg>> {
    let mut out = Vec::new();
    for part in s.split('.') {
        let (key, mut rest) = match part.find('[') {
            Some(i) => (&part[..i], &part[i..]),
            None => (part, ""),
        };
        if !key.is_empty() {
            out.push(Seg::Key(key.into()));
        }
        while let Some(r) = rest.strip_prefix('[') {
            let end = r.find(']')?;
            out.push(Seg::Index(r[..end].parse().ok()?));
            rest = &r[end + 1..];
        }
        if !rest.is_empty() {
            return None;
        }
    }
    Some(out)
}

/// Words of the game's face mask names, longest first.
const LOOK_WORDS: [(&str, &str); 24] = [
    ("fake pointy ears", "накладные острые уши"),
    ("skull face paint", "раскраска-череп"),
    ("glitter makeup", "макияж с блёстками"),
    ("surgeon mask", "хирургическая маска"),
    ("facial hair", "борода"),
    ("withBeard", "с бородой"),
    ("clown nose", "клоунский нос"),
    ("face paint", "раскраска"),
    ("ghoul face", "лицо гуля"),
    ("thief mask", "маска вора"),
    ("sunglasses", "солнечные очки"),
    ("Sunglasses", "солнечные очки"),
    ("spectacles", "очки"),
    ("facemask", "маска"),
    ("mustache", "усы"),
    ("wrinkles", "морщины"),
    ("eyepatch", "повязка на глаз"),
    ("glasses", "очки"),
    ("monocle", "монокль"),
    ("makeup", "макияж"),
    ("raider", "рейдера"),
    ("gogges", "очки-консервы"),
    ("scars", "шрамы"),
    ("beard", "борода"),
];

/// Hat names by game key, lower case without separators and gender.
const HAT_NAMES: [(&str, &str, &str); 83] = [
    (
        "enclavesecurityoutfithelmet",
        "Шлем охраны Анклава",
        "Enclave security helmet",
    ),
    ("goggleshelm", "Очки-консервы", "Goggles"),
    ("legatehelmet", "Шлем легата", "Legate helmet"),
    ("monocle", "Монокль", "Monocle"),
    (
        "paladint60helmet",
        "Шлем паладина T-60",
        "Paladin T-60 helmet",
    ),
    ("pointyearshelm", "Острые уши", "Pointy ears"),
    (
        "powerarmor51helmet",
        "Шлем силовой брони T-51",
        "T-51 power armor helmet",
    ),
    (
        "powerarmor51helmet1",
        "Шлем силовой брони T-51",
        "T-51 power armor helmet",
    ),
    (
        "powerarmor60helmet",
        "Шлем силовой брони T-60",
        "T-60 power armor helmet",
    ),
    (
        "powerarmorx1helmet",
        "Шлем силовой брони X-01",
        "X-01 power armor helmet",
    ),
    ("prestonhat", "Шляпа Престона", "Preston's hat"),
    (
        "vaultsecurityoutfithelmet",
        "Шлем охраны убежища",
        "Vault security helmet",
    ),
    (
        "alienspacesuithelmet",
        "Шлем скафандра пришельца",
        "Alien space suit helmet",
    ),
    ("alienhelmet", "Шлем пришельца", "Alien helmet"),
    ("aviatorcap", "Шлем лётчика", "Aviator cap"),
    ("bandana", "Бандана", "Bandana"),
    (
        "baseballuniformcap",
        "Кепка бейсбольной формы",
        "Baseball uniform cap",
    ),
    ("baseballcap", "Бейсболка", "Baseball cap"),
    ("battlearmor", "Шлем боевой брони", "Battle armor helmet"),
    ("businesshat", "Деловая шляпа", "Business hat"),
    ("catears", "Кошачьи уши", "Cat ears"),
    ("chefhat", "Поварской колпак", "Chef's hat"),
    ("circlet", "Диадема", "Circlet"),
    ("combathat", "Боевой шлем", "Combat helmet"),
    ("confederatehat", "Шляпа конфедерата", "Confederate hat"),
    ("confederate", "Шляпа конфедерата", "Confederate hat"),
    ("cowboyhat", "Ковбойская шляпа", "Cowboy hat"),
    ("cowboy", "Ковбойская шляпа", "Cowboy hat"),
    ("elfhat", "Колпак эльфа", "Elf hat"),
    (
        "environment",
        "Шлем защитного костюма",
        "Environment suit helmet",
    ),
    ("footballhelmet", "Футбольный шлем", "Football helmet"),
    ("ghosthalloweenhelmet", "Маска привидения", "Ghost mask"),
    ("ghosthalloweenhat", "Маска привидения", "Ghost mask"),
    ("hazmat", "Капюшон костюма химзащиты", "Hazmat hood"),
    ("headphones", "Наушники", "Headphones"),
    ("headtowel", "Полотенце на голове", "Head towel"),
    ("headwraps", "Повязка на голове", "Head wrap"),
    ("hockeymask", "Хоккейная маска", "Hockey mask"),
    ("horrormask", "Маска ужасов", "Horror mask"),
    ("kingcrown", "Королевская корона", "King's crown"),
    ("knighthat", "Рыцарский шлем", "Knight helmet"),
    ("laurel", "Лавровый венок", "Laurel wreath"),
    ("lifeguardhat", "Шляпа спасателя", "Lifeguard hat"),
    ("lincolnhat", "Шляпа Линкольна", "Lincoln's hat"),
    ("luchador", "Маска лучадора", "Luchador mask"),
    ("majuneshairhelmet", "Причёска Ма Джун", "Ma June's hair"),
    ("mascothead", "Голова талисмана", "Mascot head"),
    (
        "metalarmorhat",
        "Шлем металлической брони",
        "Metal armor helmet",
    ),
    (
        "metalarmor",
        "Шлем металлической брони",
        "Metal armor helmet",
    ),
    ("military", "Военный шлем", "Military helmet"),
    ("militaryhat", "Военная фуражка", "Military cap"),
    ("militarycap", "Военная фуражка", "Military cap"),
    ("mortarboard", "Шапочка выпускника", "Mortarboard"),
    ("motorcyclehat", "Мотоциклетный шлем", "Motorcycle helmet"),
    ("motorcycle", "Мотоциклетный шлем", "Motorcycle helmet"),
    ("ncrrangerhelmet", "Шлем рейнджера НКР", "NCR ranger helmet"),
    ("ninja", "Маска ниндзя", "Ninja hood"),
    ("nosemustache", "Нос с усами", "Nose and mustache"),
    ("officercap", "Офицерская фуражка", "Officer cap"),
    ("officer", "Офицерская фуражка", "Officer cap"),
    ("partyhat", "Праздничный колпак", "Party hat"),
    ("pilgrimhat", "Шляпа пилигрима", "Pilgrim hat"),
    ("piratehat", "Пиратская шляпа", "Pirate hat"),
    ("policehat", "Полицейская фуражка", "Police cap"),
    ("police", "Полицейская фуражка", "Police cap"),
    ("porkpiehat", "Шляпа «пирожок»", "Porkpie hat"),
    ("powerarmor", "Шлем силовой брони", "Power armor helmet"),
    (
        "robcordsuithelmet",
        "Шлем костюма RobCo",
        "RobCo R&D suit helmet",
    ),
    (
        "robotarmorhelmet",
        "Шлем роботизированной брони",
        "Robot armor helmet",
    ),
    ("santashat", "Колпак Санты", "Santa hat"),
    ("santahat", "Колпак Санты", "Santa hat"),
    (
        "silvershroudhat",
        "Шляпа Серебряного Плаща",
        "Silver Shroud hat",
    ),
    ("skeleton", "Маска скелета", "Skeleton mask"),
    ("sleazyhat", "Шляпа пижона", "Sleazy hat"),
    ("swimsuithat", "Пляжная шляпа", "Beach hat"),
    ("syntharmorhelmet", "Шлем брони синта", "Synth armor helmet"),
    ("threedoghat", "Шляпа Тридогнайта", "Three Dog's hat"),
    ("librarianglasses", "Очки библиотекаря", "Librarian glasses"),
    ("surgeonmask", "Маска хирурга", "Surgeon mask"),
    ("bonnet", "Чепчик", "Bonnet"),
    ("hardhat", "Каска", "Hard hat"),
    ("oscarhat", "Шляпа Оскара", "Oscar's hat"),
    ("piperhat", "Кепка Пайпер", "Piper's cap"),
];
