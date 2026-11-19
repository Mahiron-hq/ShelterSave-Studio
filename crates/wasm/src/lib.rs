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

//! Browser binding of the editor core. Data crosses the boundary as JSON text.

#![forbid(unsafe_code)]

mod art;

use serde_json::{json, Value as J};
use shelter_core::actions::{self, Action};
use shelter_core::catalog::{catalog, ItemKind};
use shelter_core::document::{Document, Session};
use shelter_core::i18n::Lang;
use shelter_core::json::path_to_string;
use shelter_core::map;
use shelter_core::rules::Mode;
use shelter_core::text::{parse_path, Tx};
use wasm_bindgen::prelude::*;

fn err(code: &str, text: String) -> JsError {
    JsError::new(&json!({ "code": code, "text": text }).to_string())
}

#[wasm_bindgen]
pub struct Editor {
    doc: Option<Document>,
    mode: Mode,
    tx: Tx,
}

impl Default for Editor {
    fn default() -> Self {
        Editor::new()
    }
}

#[wasm_bindgen]
impl Editor {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Editor {
        Editor {
            doc: None,
            mode: Mode::Normal,
            tx: Tx::new(Lang::En, false),
        }
    }

    /// Interface language ("en" | "ru"), item names in Russian, expert mode.
    pub fn prefs(&mut self, lang: &str, item_ru: bool, expert: bool) {
        self.tx = Tx::new(Lang::from_locale(lang), item_ru);
        self.mode = if expert { Mode::Expert } else { Mode::Normal };
    }

    pub fn open(&mut self, bytes: &[u8], name: &str) -> Result<(), JsError> {
        match Document::open(bytes, name) {
            Ok(doc) => {
                self.doc = Some(doc);
                Ok(())
            }
            Err(e) => Err(err(
                e.code(),
                self.tx
                    .tf(&format!("err.{}", e.code()), &[("detail", String::new())]),
            )),
        }
    }

    pub fn close(&mut self) {
        self.doc = None;
    }

    #[wasm_bindgen(js_name = isOpen)]
    pub fn is_open(&self) -> bool {
        self.doc.is_some()
    }

    #[wasm_bindgen(js_name = fileName)]
    pub fn file_name(&self) -> String {
        self.doc
            .as_ref()
            .map(|d| d.file_name.clone())
            .unwrap_or_default()
    }

    pub fn revision(&self) -> f64 {
        self.doc.as_ref().map_or(0.0, |d| d.revision() as f64)
    }

    pub fn dirty(&self) -> bool {
        self.doc.as_ref().is_some_and(Document::is_dirty)
    }

    /// Current tree as compact JSON.
    pub fn root(&self) -> String {
        self.doc
            .as_ref()
            .map(|d| d.root().to_compact())
            .unwrap_or_default()
    }

    /// Value at `a.b[2]` in the original file, as JSON text.
    pub fn original(&self, path: &str) -> Option<String> {
        let doc = self.doc.as_ref()?;
        doc.original_at(&parse_path(path)?).map(|v| v.to_compact())
    }

    /// Current value at a path as JSON text (numbers keep their literal).
    pub fn raw(&self, path: &str) -> Option<String> {
        let doc = self.doc.as_ref()?;
        doc.root().at(&parse_path(path)?).map(|v| v.to_compact())
    }

    pub fn modified(&self, path: &str) -> bool {
        match (self.doc.as_ref(), parse_path(path)) {
            (Some(d), Some(p)) => d.is_modified(&p),
            _ => false,
        }
    }

    /// Runs an action given as JSON; returns `{label, affected}`.
    pub fn apply(&mut self, action: &str) -> Result<String, JsError> {
        let action: Action =
            serde_json::from_str(action).map_err(|e| err("bad_action", e.to_string()))?;
        let Some(doc) = self.doc.as_mut() else {
            return Err(err("no_file", String::new()));
        };
        match doc.apply(self.mode, &action) {
            Ok(tx) => Ok(json!({
                "label": self.tx.label_text(&tx.label, Some(doc.root())),
                "affected": tx.affected,
            })
            .to_string()),
            Err(shelter_core::actions::ActionError::Nothing) => Err(err("nothing", String::new())),
            Err(e) => Err(err("action", self.tx.error_text(&e))),
        }
    }

    pub fn undo(&mut self) -> Result<Option<String>, JsError> {
        let Some(doc) = self.doc.as_mut() else {
            return Ok(None);
        };
        let Some(label) = doc.undo().map_err(|e| err("edit", e.to_string()))? else {
            return Ok(None);
        };
        Ok(Some(self.tx.label_text(&label, Some(doc.root()))))
    }

    pub fn redo(&mut self) -> Result<Option<String>, JsError> {
        let Some(doc) = self.doc.as_mut() else {
            return Ok(None);
        };
        let Some(label) = doc.redo().map_err(|e| err("edit", e.to_string()))? else {
            return Ok(None);
        };
        Ok(Some(self.tx.label_text(&label, Some(doc.root()))))
    }

    #[wasm_bindgen(js_name = goTo)]
    pub fn go_to(&mut self, step: usize) -> bool {
        self.doc.as_mut().is_some_and(|d| d.go_to(step).is_ok())
    }

    #[wasm_bindgen(js_name = resetToOriginal)]
    pub fn reset_to_original(&mut self) -> bool {
        self.doc
            .as_mut()
            .is_some_and(|d| d.reset_to_original().is_some())
    }

    /// Journal: `{done: [{text, affected}], undone: [text]}`, oldest first.
    pub fn history(&self) -> String {
        let Some(doc) = self.doc.as_ref() else {
            return "{}".into();
        };
        let h = doc.history();
        let root = Some(doc.root());
        let done: Vec<J> = h
            .done()
            .iter()
            .map(|t| json!({ "text": self.tx.label_text(&t.label, root), "affected": t.affected }))
            .collect();
        let undone: Vec<String> = h
            .undone()
            .map(|t| self.tx.label_text(&t.label, root))
            .collect();
        json!({ "done": done, "undone": undone, "canUndo": h.can_undo(), "canRedo": h.can_redo() })
            .to_string()
    }

    /// Changes, integrity issues and warnings, with readable places.
    #[wasm_bindgen(js_name = saveReport)]
    pub fn save_report(&self) -> String {
        let Some(doc) = self.doc.as_ref() else {
            return "{}".into();
        };
        let r = doc.save_report();
        let root = doc.root();
        let original = doc.original();
        let changes: Vec<J> = r
            .changes
            .iter()
            .map(|c| {
                let tree = if c.after.is_none() { original } else { root };
                json!({
                    "kind": c.kind,
                    "path": path_to_string(&c.path),
                    "place": self.tx.path_title(tree, &c.path),
                    "before": c.before,
                    "after": c.after,
                })
            })
            .collect();
        let issues = |list: &[shelter_core::integrity::Issue]| self.issue_list(list);
        let warnings = |list: &[shelter_core::rules::Warning]| -> Vec<J> {
            list.iter()
                .map(|w| {
                    json!({
                        "code": w.code,
                        "path": path_to_string(&w.path),
                        "place": self.tx.path_title(root, &w.path),
                        "value": w.value,
                        "min": w.min,
                        "max": w.max,
                    })
                })
                .collect()
        };
        json!({
            "changes": changes,
            "newIssues": issues(&r.new_issues),
            "oldIssues": issues(&r.old_issues),
            "newWarnings": warnings(&r.new_warnings),
            "oldWarnings": warnings(&r.old_warnings),
        })
        .to_string()
    }

    /// Integrity issues: `{new: [...], old: [...]}`.
    pub fn issues(&self) -> String {
        let Some(doc) = self.doc.as_ref() else {
            return "{}".into();
        };
        let (new, old) = doc.issues();
        json!({ "new": self.issue_list(&new), "old": self.issue_list(&old) }).to_string()
    }

    /// Encrypted `.sav` text, or pretty JSON when the file was opened as JSON.
    pub fn save(&mut self) -> String {
        let Some(doc) = self.doc.as_mut() else {
            return String::new();
        };
        let out = match doc.source {
            shelter_core::document::SourceKind::Sav => doc.to_sav(),
            shelter_core::document::SourceKind::Json => doc.to_json_pretty(),
        };
        doc.mark_saved();
        out
    }

    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> String {
        self.doc
            .as_ref()
            .map(Document::to_json_pretty)
            .unwrap_or_default()
    }

    /// Session for the local cache.
    pub fn session(&self) -> Option<String> {
        serde_json::to_string(&self.doc.as_ref()?.session()).ok()
    }

    pub fn restore(&mut self, session: &str) -> Result<(), JsError> {
        let s: Session =
            serde_json::from_str(session).map_err(|e| err("bad_session", e.to_string()))?;
        let doc = Document::restore(s).map_err(|e| err(e.code(), String::new()))?;
        self.doc = Some(doc);
        Ok(())
    }

    /// Helpers that read the open save, by name, with JSON arguments.
    pub fn query(&self, name: &str, args: &str) -> String {
        let Some(doc) = self.doc.as_ref() else {
            return "null".into();
        };
        let a: J = serde_json::from_str(args).unwrap_or(J::Null);
        let root = doc.root();
        let int = |k: &str| a.get(k).and_then(J::as_i64).unwrap_or(0);
        let text = |k: &str| a.get(k).and_then(J::as_str).unwrap_or("").to_string();
        let at = |k: &str| {
            a.get(k)
                .and_then(J::as_str)
                .and_then(parse_path)
                .and_then(|p| root.at(&p).cloned())
        };
        let out = match name {
            "themeState" => json!(actions::theme_state(root, &text("id"))),
            "mergeCol" => json!(actions::merge_col(
                root,
                self.mode,
                int("room"),
                int("merge")
            )),
            "roomLock" => json!(actions::room_lock(root, &text("kind"))),
            "roomBusy" => json!(at("path").is_some_and(|r| actions::room_busy(&r))),
            "roomMaxHealth" => json!(at("path").and_then(|r| actions::room_max_health(&r))),
            "teamLoad" => json!(at("path").map(|t| actions::team_load(&t))),
            "map" => {
                let (rows, cols) = map::grid_size();
                let burning: Vec<i64> = shelter_core::vault::arr(root, &shelter_core::vault::ROOMS)
                    .iter()
                    .filter(|r| {
                        shelter_core::vault::text(r, &["currentStateName"]).as_deref()
                            == Some("RoomEmergency")
                    })
                    .filter_map(|r| shelter_core::vault::int(r, &["deserializeID"]))
                    .collect();
                let rooms: Vec<J> = map::rooms(root)
                    .iter()
                    .map(|r| {
                        json!({
                            "id": r.id, "kind": r.kind, "row": r.row, "col": r.col,
                            "width": r.width, "merge": r.merge, "level": r.level,
                        })
                    })
                    .collect();
                json!({ "rows": rows, "cols": cols, "rooms": rooms, "rocks": map::obstacles(root), "burning": burning })
            }
            "checkSpot" => {
                let id = a.get("id").and_then(J::as_i64);
                match map::check_spot(
                    root,
                    &text("kind"),
                    int("row"),
                    int("col"),
                    int("merge"),
                    id,
                ) {
                    Ok(()) => J::Null,
                    Err(e) => json!(e),
                }
            }
            "fits" => {
                let id = a.get("id").and_then(J::as_i64);
                match map::fits(root, int("row"), int("col"), int("width"), id) {
                    Ok(()) => J::Null,
                    Err(e) => json!(e),
                }
            }
            "addCol" => json!(map::add_col(
                root,
                &text("kind"),
                int("row"),
                int("at"),
                int("merge"),
                a.get("rock").and_then(J::as_bool).unwrap_or(false),
            )),
            "canRemove" => json!(map::can_remove(root, int("id"))),
            "outfitLook" => {
                use shelter_core::vault::{arr, flag, int as vint, DWELLERS};
                let Some(d) = arr(root, &DWELLERS).get(int("index") as usize) else {
                    return "null".into();
                };
                let gender = vint(d, &["gender"]).unwrap_or(1);
                let male = gender == 2;
                let worn = if !male && flag(d, &["pregnant"]) == Some(true) {
                    Some("__pregnant".to_string())
                } else {
                    d.get("equipedOutfit")
                        .and_then(|o| o.get("id"))
                        .and_then(shelter_core::json::Value::as_text)
                };
                let key = if male { "male" } else { "female" };
                let art = worn
                    .as_deref()
                    .and_then(|w| catalog().art.as_ref()?.dweller.get(key)?.outfits.get(w));
                let color = vint(d, &["outfitColor"]).unwrap_or(0xFFFF_FFFF) as u32;
                json!({
                    "worn": worn,
                    "known": art.is_some(),
                    "fits": worn.as_deref().is_none_or(|w| catalog().outfit_fits(w, gender)),
                    "colors": art.map(|o| o.colors.clone()).unwrap_or_default(),
                    "used": art.map_or(0xFFFF_FFFF, |o| o.game_color(color)),
                    "largeHeadgear": art.is_some_and(|o| o.large_headgear),
                    "hasArt": catalog().art.is_some(),
                })
            }
            "copySpot" => {
                let id = int("id");
                match map::rooms(root).into_iter().find(|r| r.id == id) {
                    Some(me) => {
                        let ((row, col), ok) = map::copy_spot(root, &me);
                        json!({ "row": row, "col": col, "ok": ok })
                    }
                    None => J::Null,
                }
            }
            "room" => {
                let id = int("id");
                let Some(i) = shelter_core::vault::room_index(root, id) else {
                    return "null".into();
                };
                let r = &shelter_core::vault::arr(root, &shelter_core::vault::ROOMS)[i];
                let kind = shelter_core::vault::text(r, &["type"]).unwrap_or_default();
                let merge = shelter_core::vault::int(r, &["mergeLevel"]).unwrap_or(1);
                let level = shelter_core::vault::int(r, &["level"]).unwrap_or(1);
                let sizes: Vec<J> = map::merges(&kind)
                    .into_iter()
                    .chain(std::iter::once(merge))
                    .collect::<std::collections::BTreeSet<_>>()
                    .into_iter()
                    .map(|m| {
                        json!({
                            "merge": m,
                            "fits": actions::merge_fits(root, self.mode, id, m),
                            "free": actions::merge_fits(root, Mode::Expert, id, m),
                        })
                    })
                    .collect();
                let neighbours = map::placed(r)
                    .is_some_and(|me| !map::merge_neighbours(&map::rooms(root), &me).is_empty());
                let (pk, pid) = shelter_core::pictures::room_picture(root, r);
                json!({
                    "index": i,
                    "sizes": sizes,
                    "maxLevel": map::max_level(&kind, merge),
                    "neighbours": neighbours,
                    "canRemove": map::can_remove(root, id),
                    "busy": actions::room_busy(r),
                    "batch": catalog().room(&kind).and_then(|info| info.batch(merge, level)),
                    "product": map::room_product(&kind),
                    "picture": [pk, pid],
                    "pictureAlt": shelter_core::pictures::room_picture_fallback(&pid),
                    "lock": actions::room_lock(root, &kind),
                    "theme": shelter_core::pictures::applied_theme(root, &kind),
                })
            }
            _ => J::Null,
        };
        out.to_string()
    }

    /// Room type name at a level, in the interface language.
    #[wasm_bindgen(js_name = kindName)]
    pub fn kind_name(&self, kind: &str, level: i32) -> String {
        self.tx.kind_name(kind, i64::from(level))
    }

    /// Names a room type has at its levels, without repeats.
    #[wasm_bindgen(js_name = levelNames)]
    pub fn level_names(&self, kind: &str) -> Vec<String> {
        self.tx.level_names(kind)
    }

    /// Hair, beard, mask or hat choices of a gender with readable names.
    #[wasm_bindgen(js_name = lookOptions)]
    pub fn look_options(&self, gender: i32, field: &str) -> String {
        let list: Vec<J> = actions::look_choices(i64::from(gender), field)
            .into_iter()
            .map(|n| {
                let name = if field == "helmet" {
                    self.tx.hat_name(n)
                } else {
                    None
                };
                json!({ "value": n, "label": name.unwrap_or_else(|| self.tx.look_name(n)) })
            })
            .collect();
        J::Array(list).to_string()
    }

    #[wasm_bindgen(js_name = questLine)]
    pub fn quest_line(&self, id: &str) -> String {
        self.tx.quest_line(id)
    }

    #[wasm_bindgen(js_name = gameDate)]
    pub fn game_date(&self, s: &str) -> String {
        self.tx.game_date(s)
    }

    /// Wasteland journal line: [time, text].
    #[wasm_bindgen(js_name = logLine)]
    pub fn log_line(&self, code: &str) -> Vec<String> {
        let (t, x) = self.tx.log_line(code);
        vec![t, x]
    }

    #[wasm_bindgen(js_name = incidentName)]
    pub fn incident_name(&self, mode: i32) -> String {
        self.tx.incident_name(i64::from(mode))
    }

    /// Objective text at a level; `req` is the saved requirement JSON or empty.
    #[wasm_bindgen(js_name = objectiveText)]
    pub fn objective_text(&self, id: &str, level: i32, req: &str) -> String {
        let req = shelter_core::json::parse(req).ok();
        self.tx.objective_text(id, i64::from(level), req.as_ref())
    }

    #[wasm_bindgen(js_name = rewardText)]
    pub fn reward_text(&self, id: &str, level: i32) -> String {
        catalog()
            .objective(id)
            .map(|o| self.tx.reward_text(o, i64::from(level)))
            .unwrap_or_default()
    }

    /// Objectives that may replace one at `level`: `[{id, text, tier, reward, level}]`.
    #[wasm_bindgen(js_name = objectiveOptions)]
    pub fn objective_options(
        &self,
        level: i32,
        survival: bool,
        used: Vec<String>,
        current: &str,
    ) -> String {
        let expert = self.mode == Mode::Expert;
        let list: Vec<J> = catalog()
            .objectives
            .iter()
            .filter(|o| {
                expert || (!o.tutorial && !o.season && if survival { o.survival } else { o.normal })
            })
            .filter(|o| o.id == current || !used.contains(&o.id))
            .map(|o| {
                let lvl = o.level_used(i64::from(level));
                let params = self.tx.objective_params(o, None, o.goal_value(lvl));
                json!({
                    "id": o.id,
                    "text": o.text(self.tx.ru(), lvl, &params),
                    "tier": o.level,
                    "reward": self.tx.reward_text(o, lvl),
                    "level": lvl,
                })
            })
            .collect();
        J::Array(list).to_string()
    }

    /// Highest difficulty level an objective can be set to, if it grows.
    #[wasm_bindgen(js_name = objectiveMaxLevel)]
    pub fn objective_max_level(id: &str) -> Option<i32> {
        let o = catalog().objective(id)?;
        let g = o.goal.as_ref().filter(|g| g.step != 0)?;
        g.max_level.filter(|m| *m > 0).map(|m| m as i32)
    }

    pub fn duration(&self, secs: f64) -> String {
        self.tx.duration(secs)
    }

    /// Readable place for a path like `dwellers.dwellers[3].happiness`.
    #[wasm_bindgen(js_name = pathTitle)]
    pub fn path_title(&self, path: &str) -> String {
        match (self.doc.as_ref(), parse_path(path)) {
            (Some(d), Some(p)) => self.tx.path_title(d.root(), &p),
            _ => path.to_string(),
        }
    }
}

impl Editor {
    fn issue_list(&self, list: &[shelter_core::integrity::Issue]) -> Vec<J> {
        list.iter()
            .map(|i| {
                json!({
                    "code": i.code,
                    "severity": i.severity,
                    "path": path_to_string(&i.path),
                    "segs": i.path,
                    "text": self.tx.issue_text(i),
                    "fixable": i.fixable,
                })
            })
            .collect()
    }
}

/// Reference items of the given kinds (comma separated) matching a query.
/// `gender` 1/2 keeps only outfits that gender can wear; 0 keeps all.
#[wasm_bindgen]
pub fn search_items(kinds: &str, query: &str, gender: i32) -> String {
    let kinds: Vec<ItemKind> = kinds.split(',').filter_map(ItemKind::parse).collect();
    let found: Vec<J> = catalog()
        .search(&kinds, query)
        .into_iter()
        .filter(|i| {
            gender == 0
                || i.kind != ItemKind::Outfit
                || catalog().outfit_fits(i.id, i64::from(gender))
        })
        .map(|i| json!({ "kind": i.kind.as_str(), "id": i.id, "rarity": i.rarity, "name": i.name }))
        .collect();
    J::Array(found).to_string()
}

#[wasm_bindgen]
pub fn outfit_fits(id: &str, gender: i32) -> bool {
    catalog().outfit_fits(id, i64::from(gender))
}

/// Sizes of a room type and its highest level at each size.
#[wasm_bindgen]
pub fn room_shape(kind: &str) -> String {
    let merges = map::merges(kind);
    let levels: Vec<i64> = merges.iter().map(|m| map::max_level(kind, *m)).collect();
    json!({ "merges": merges, "maxLevel": levels, "width": merges.iter().map(|m| map::width_of(kind, *m)).collect::<Vec<_>>() })
        .to_string()
}

/// Picture key of a theme (or the standard look) for a room type.
#[wasm_bindgen]
pub fn theme_picture(room: &str, theme: Option<String>) -> String {
    json!(shelter_core::pictures::theme_picture(
        room,
        theme.as_deref()
    ))
    .to_string()
}

/// Hair, beard, mask or hat choices of a gender, as the game offers them.
#[wasm_bindgen]
pub fn look_choices(gender: i32, field: &str) -> String {
    json!(actions::look_choices(i64::from(gender), field)).to_string()
}

/// Dwellers of another save file (only read), for bringing them over.
#[wasm_bindgen]
pub fn read_dwellers(bytes: &[u8]) -> Result<String, JsError> {
    let doc = Document::open(bytes, "").map_err(|e| err(e.code(), String::new()))?;
    let list: Vec<J> = shelter_core::vault::arr(doc.root(), &shelter_core::vault::DWELLERS)
        .iter()
        .map(|d| {
            json!({
                "name": shelter_core::vault::dweller_name(d),
                "level": shelter_core::vault::int(d, &["experience", "currentLevel"]).unwrap_or(1),
                "rarity": shelter_core::vault::text(d, &["rarity"]).unwrap_or_default(),
                "json": d.to_compact(),
            })
        })
        .collect();
    Ok(J::Array(list).to_string())
}

/// Legendary dwellers listed in the game journal.
#[wasm_bindgen]
pub fn journal_dwellers() -> String {
    let list: Vec<J> = catalog()
        .unique_dwellers
        .iter()
        .filter(|u| u.in_journal())
        .map(|u| json!({ "id": u.id, "first": u.first_name, "last": u.last_name, "outfit": u.outfit, "weapon": u.weapon, "special": u.special }))
        .collect();
    J::Array(list).to_string()
}

/// Game catalog as JSON.
#[wasm_bindgen]
pub fn catalog_json() -> String {
    shelter_core::catalog::CATALOG_JSON.to_string()
}

/// Limits and key lists the interface needs.
#[wasm_bindgen]
pub fn consts() -> String {
    use shelter_core::{actions, map, rules, vault};
    json!({
        "maxDwellers": rules::MAX_DWELLERS,
        "resourceMax": rules::RESOURCE_MAX,
        "lunchboxMax": rules::LUNCHBOX_MAX,
        "itemAddMax": rules::ITEM_ADD_MAX,
        "rarities": rules::RARITIES,
        "resourceKeys": vault::RESOURCE_KEYS,
        "playerResources": vault::PLAYER_RESOURCES,
        "themeParts": actions::THEME_PARTS,
        "roomIncidents": actions::ROOM_INCIDENTS,
        "noIncidentRooms": actions::NO_INCIDENT_ROOMS,
        "fixedRooms": map::FIXED_ROOMS,
        "notBuildable": map::NOT_BUILDABLE,
        "uniqueRooms": map::UNIQUE_ROOMS,
        "questMemory": actions::QUEST_MEMORY,
        "themeRooms": shelter_core::pictures::THEME_ROOMS,
        "holidays": shelter_core::pictures::HOLIDAYS,
    })
    .to_string()
}

/// Interface strings for a language.
#[wasm_bindgen]
pub fn strings(lang: &str) -> String {
    match Lang::from_locale(lang) {
        Lang::Ru => shelter_core::i18n::RU_JSON.to_string(),
        Lang::En => shelter_core::i18n::EN_JSON.to_string(),
    }
}

/// Label and help of a field path.
#[wasm_bindgen]
pub fn describe(lang: &str, path: &str) -> String {
    let f = shelter_core::fields::describe(
        Lang::from_locale(lang),
        &parse_path(path).unwrap_or_default(),
    );
    json!({ "label": f.label, "help": f.help, "known": f.known }).to_string()
}

/// .NET ticks (decimal text, too big for a JS number) as date and time.
#[wasm_bindgen]
pub fn ticks_format(ticks: &str) -> Option<String> {
    shelter_core::ticks::format(ticks.trim().parse().ok()?)
}

/// Parses date and time text back to ticks, keeping the sub-second part.
#[wasm_bindgen]
pub fn ticks_parse(text: &str, keep: &str) -> Option<String> {
    shelter_core::ticks::parse(text, keep.trim().parse().ok()?).map(|t| t.to_string())
}

/// Keys stored as 0/1 that the editor shows as yes/no.
#[wasm_bindgen]
pub fn is_int_flag(key: &str) -> bool {
    shelter_core::fields::is_int_flag(key)
}
