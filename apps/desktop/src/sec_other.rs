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

use eframe::egui::{self, RichText};
use shelter_core::actions::Action;
use shelter_core::catalog::{catalog, ItemKind, Objective};
use shelter_core::i18n::Lang;
use shelter_core::json::{path_to_string, Seg, Value};
use shelter_core::vault::{self, p, pi};

use crate::app::{panel, section_title, App, OtherTab};
use crate::icons;
use crate::style;
use crate::widgets::{self, Opts};

const SERVICE_KNOWN: [&str; 41] = [
    "timeMgr",
    "localNotificationMgr",
    "taskMgr",
    "ratingMgr",
    "specialTheme",
    "dwellers",
    "constructMgr",
    "vault",
    "dwellerSpawner",
    "deviceName",
    "tutorialManager",
    "objectiveMgr",
    "unlockableMgr",
    "survivalW",
    "ShopWindow",
    "happinessManager",
    "refugeeSpawner",
    "LunchBoxCollectWindow",
    "SeasonShopWarningModal",
    "DeathclawManager",
    "PromoCodesWindow",
    "JunkGiveAwayWindow",
    "MysteriousStranger",
    "StatsWindow",
    "PromotionFlags",
    "appVersion",
    "BottleAndCappyMgrSerializeKey",
    "completedQuestDataManager",
    "seasonEventData",
    "dayToDayRewardMgr",
    "seasonId",
    "cameraPosition",
    "questSetup",
    "questDataManager",
    "questDwellers",
    "questDwellerSpawner",
    "task",
    "time",
    "viewManager",
    "versionCount",
    "swrveEventsManager",
];

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    section_title(ui, icons::LAYERS, &app.t("nav.other"), "");
    let expert = app.expert();
    if !expert && matches!(app.other_tab, OtherTab::Service | OtherTab::Tree) {
        app.other_tab = OtherTab::Quests;
    }
    let mut tabs = vec![
        (OtherTab::Quests, "o.quests"),
        (OtherTab::Wasteland, "o.wasteland"),
        (OtherTab::Objectives, "o.objectives"),
        (OtherTab::Stats, "o.stats"),
        (OtherTab::Tutorial, "o.tutorial"),
        (OtherTab::Events, "o.events"),
    ];
    if expert {
        tabs.push((OtherTab::Service, "o.service"));
        tabs.push((OtherTab::Tree, "o.tree"));
    }
    ui.horizontal_wrapped(|ui| {
        for (tab, key) in tabs {
            if ui
                .selectable_label(app.other_tab == tab, app.t(key))
                .clicked()
            {
                app.other_tab = tab;
            }
        }
    });
    ui.separator();
    if app.other_tab == OtherTab::Tree {
        crate::tree::show(app, ui);
        return;
    }
    egui::ScrollArea::vertical()
        .id_salt(("other", app.other_tab as u8))
        .auto_shrink(false)
        .show(ui, |ui| match app.other_tab {
            OtherTab::Quests => quests(app, ui),
            OtherTab::Wasteland => wasteland(app, ui),
            OtherTab::Objectives => objectives(app, ui),
            OtherTab::Stats => stats(app, ui),
            OtherTab::Tutorial => tutorial(app, ui),
            OtherTab::Events => events(app, ui),
            OtherTab::Service => service(app, ui),
            OtherTab::Tree => {}
        });
}

fn root(app: &App) -> std::sync::Arc<Value> {
    app.open
        .as_ref()
        .map_or_else(|| std::sync::Arc::new(Value::Null), |o| o.doc.shared_root())
}

fn title(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).font(style::bold(16.0)));
}

fn note(ui: &mut egui::Ui, text: &str) {
    let c = style::colors(ui.ctx());
    ui.add(egui::Label::new(RichText::new(text).color(c.muted)).wrap());
}

/// Read-only line: muted name, then the value.
fn info(ui: &mut egui::Ui, name: &str, value: &str) {
    let c = style::colors(ui.ctx());
    if ui.available_width() < 460.0 {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(name).color(c.muted));
            ui.label(value);
        });
        return;
    }
    // Same columns as widgets::row, so values line up with editors.
    ui.horizontal_top(|ui| {
        ui.add_space(8.0);
        let w = widgets::LABEL_COL.min(ui.available_width() * 0.4);
        ui.allocate_ui_with_layout(
            egui::vec2(w, 0.0),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                ui.set_width(w);
                ui.label(RichText::new(name).color(c.muted));
            },
        );
        ui.add(egui::Label::new(value).wrap());
    });
}

fn join(base: &[Seg], key: &str) -> Vec<Seg> {
    let mut out = base.to_vec();
    out.push(Seg::Key(key.into()));
    out
}

fn quest_line(app: &App, id: &str) -> String {
    app.tx().quest_line(id)
}

fn game_date(app: &App, s: &str) -> String {
    app.tx().game_date(s)
}

fn dweller_name_any(root: &Value, id: i64) -> String {
    for list in [&["questDwellers", "dwellers"][..], &vault::DWELLERS[..]] {
        if let Some(d) = vault::arr(root, list)
            .iter()
            .find(|d| vault::int(d, &["serializeId"]) == Some(id))
        {
            return vault::dweller_name(d);
        }
    }
    format!("#{id}")
}

fn duration(app: &App, secs: f64) -> String {
    app.tx().duration(secs)
}

fn quests(app: &mut App, ui: &mut egui::Ui) {
    let r = root(app);
    panel(ui, |ui| {
        title(ui, &app.t("o.currentQuest"));
        let id = r
            .at(&p(&["questDataManager", "questTeam", "CurrentQuestID"]))
            .and_then(Value::as_text)
            .unwrap_or_default();
        if id.is_empty() {
            note(ui, &app.t("o.noQuest"));
            return;
        }
        let flag = |k: &str| {
            r.at(&p(&["questDataManager", k]))
                .and_then(Value::as_bool)
                .unwrap_or(false)
        };
        let status = if flag("cancelled") {
            app.t("o.quest.cancelled")
        } else if flag("questDone") && flag("questSucceeded") {
            app.t("o.quest.succeeded")
        } else if flag("questDone") {
            app.t("o.quest.failed")
        } else {
            app.t("o.quest.running")
        };
        info(ui, &app.t("o.quest"), &quest_line(app, &id));
        info(ui, &app.t("o.status"), &status);
        let team: Vec<String> =
            vault::arr(&r, &["questDataManager", "questTeam", "DwellersDictionary"])
                .iter()
                .filter_map(Value::as_i64)
                .map(|d| dweller_name_any(&r, d))
                .collect();
        if !team.is_empty() {
            info(ui, &app.t("o.team"), &team.join(", "));
        }
        if !flag("questDone") && !flag("cancelled") {
            ui.add_space(4.0);
            for (key, label, help) in [
                ("currentStimpakCount", "o.questStim", "o.questStimHelp"),
                ("currentRadawayCount", "o.questRad", "o.questRadHelp"),
            ] {
                let path = p(&["questDataManager", "questTeam", key]);
                if r.at(&path).is_some() {
                    let o = Opts::text(app.t(label)).help(app.t(help)).range(0.0, 25.0);
                    widgets::num_field(app, ui, &path, o);
                }
            }
            let team = vault::arr(&r, &["questDwellers", "dwellers"]);
            if !team.is_empty() {
                ui.add_space(6.0);
                widgets::hint_label(
                    ui,
                    &app.t("o.questTeamHealth"),
                    &app.t("o.questTeamHealthHelp"),
                );
            }
            for (i, d) in team.iter().enumerate() {
                let max = d
                    .at(&p(&["health", "maxHealth"]))
                    .and_then(Value::as_f64)
                    .unwrap_or(100.0)
                    .max(1.0);
                ui.label(RichText::new(vault::dweller_name(d)).strong());
                for (key, label) in [
                    ("healthValue", "o.questHp"),
                    ("radiationValue", "o.questRads"),
                ] {
                    let path = pi(&["questDwellers", "dwellers"], i, &["health", key]);
                    if r.at(&path).is_some() {
                        let o = Opts::text(app.t(label)).range(0.0, max);
                        widgets::num_field(app, ui, &path, o);
                    }
                }
            }
        }
    });
    ui.add_space(8.0);
    let mut replace = None;
    panel(ui, |ui| {
        title(ui, &app.t("o.dailyWeekly"));
        note(ui, &app.t("o.dailyWeeklyHelp"));
        let running = r
            .at(&p(&["questDataManager", "questTeam", "CurrentQuestID"]))
            .and_then(Value::as_text)
            .filter(|_| {
                !r.at(&p(&["questDataManager", "questDone"]))
                    .and_then(Value::as_bool)
                    .unwrap_or(true)
            });
        let mut rows: Vec<(&str, usize, &str, String, Option<String>)> = Vec::new();
        for (kind, list, label) in [
            (
                "daily",
                &[
                    "completedQuestDataManager",
                    "dailyQuestPicker",
                    "currentDailies",
                ][..],
                "o.daily",
            ),
            (
                "weekly",
                &[
                    "completedQuestDataManager",
                    "weeklyQuestPicker",
                    "currentWeeklies",
                ][..],
                "o.weekly",
            ),
        ] {
            for (i, q) in vault::arr(&r, list).iter().enumerate() {
                if let Some(name) = vault::text(q, &["questName"]) {
                    rows.push((kind, i, label, name, vault::text(q, &["endDate"])));
                }
            }
        }
        if let Some(name) = vault::text(
            &r,
            &[
                "completedQuestDataManager",
                "standaloneQuestPicker",
                "currentStandalone",
            ],
        ) {
            rows.push(("standalone", 0, "o.standalone", name, None));
        }
        if rows.is_empty() {
            note(ui, &app.t("o.noDaily"));
        }
        for (kind, i, label, name, end) in rows {
            ui.add_space(4.0);
            let mut line = quest_line(app, &name);
            if let Some(end) = end {
                let end = game_date(app, &end);
                line = format!("{line}  ·  {}", app.tf("o.until", &[("date", end)]));
            }
            info(ui, &app.t(label), &line);
            if running.as_deref() == Some(name.as_str()) {
                note(ui, &app.t("o.questRunningNoReplace"));
                continue;
            }
            let prefix = match kind {
                "daily" => "Daily_",
                "weekly" => "Weekly_",
                _ => "Standalone_",
            };
            if let Some(id) = quest_picker(app, ui, &format!("{kind}{i}"), prefix, &name) {
                replace = Some((kind.to_string(), i, id));
            }
        }
    });
    if let Some((kind, index, id)) = replace {
        app.act(Action::ReplaceQuest { kind, index, id });
    }
    ui.add_space(8.0);
    quest_list(app, ui, &r);
    ui.add_space(8.0);
    quest_memory(app, ui, &r);
}

/// Quests the game will not offer again for now: recent ones and skipped ones.
fn quest_memory(app: &mut App, ui: &mut egui::Ui, r: &Value) {
    use shelter_core::actions::QUEST_MEMORY;
    let c = style::colors(ui.ctx());
    let mut forget: Option<(String, String)> = None;
    panel(ui, |ui| {
        title(ui, &app.t("o.questMemory"));
        note(ui, &app.t("o.questMemoryHelp"));
        for (key, path) in QUEST_MEMORY {
            let list = vault::arr(r, &path);
            if list.is_empty() {
                continue;
            }
            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(app.t(&format!("o.qm.{key}"))).strong());
                if ui.small_button(app.t("o.qmClear")).clicked() {
                    forget = Some((key.to_string(), String::new()));
                }
            });
            for e in list {
                let Some(name) = vault::text(e, &["questName"]) else {
                    continue;
                };
                ui.horizontal_wrapped(|ui| {
                    if crate::app::small_icon(ui, icons::X, &app.t("o.qmForget")).clicked() {
                        forget = Some((key.to_string(), name.clone()));
                    }
                    ui.label(quest_line(app, &name));
                    if let Some(d) = vault::text(e, &["startDate"]) {
                        ui.label(RichText::new(game_date(app, &d)).small().color(c.muted));
                    }
                });
            }
        }
        let clues = vault::arr(r, &["completedQuestDataManager", "foundClues"]);
        if !clues.is_empty() {
            ui.add_space(6.0);
            let names: Vec<String> = clues.iter().filter_map(Value::as_text).collect();
            info(ui, &app.t("o.questClues"), &names.join(", "));
        }
    });
    if let Some((list, quest)) = forget {
        app.act(Action::ForgetQuest { list, quest });
    }
}

/// "Replace" button with a searchable list of quests whose id starts with `prefix`.
fn quest_picker(
    app: &mut App,
    ui: &mut egui::Ui,
    salt: &str,
    prefix: &str,
    current: &str,
) -> Option<String> {
    let c = style::colors(ui.ctx());
    let expert = app.expert();
    let key = format!("quest_pick_{salt}");
    let mut query = app.bufs.get(&key).cloned().unwrap_or_default();
    let mut picked = None;
    let btn = ui
        .button(format!("{}  {}", icons::PENCIL, app.t("o.replaceQuest")))
        .on_hover_text(app.t("o.replaceQuestHelp"));
    egui::Popup::menu(&btn)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| {
            ui.set_min_width(420.0);
            ui.horizontal(|ui| {
                ui.label(icons::SEARCH);
                ui.add(
                    egui::TextEdit::singleline(&mut query)
                        .hint_text(app.t("c.search"))
                        .desired_width(360.0),
                );
            });
            let q = query.to_lowercase();
            let list: Vec<(String, String)> = catalog()
                .quests
                .iter()
                .filter(|x| x.id.starts_with(prefix))
                .map(|x| {
                    let mut name = quest_line(app, &x.id);
                    // Ids like Daily_04_Diff_10_B differ only by the last letter.
                    if let Some(v) =
                        x.id.rsplit('_')
                            .next()
                            .filter(|v| v.len() == 1 && v.chars().all(|ch| ch.is_ascii_uppercase()))
                    {
                        name = format!(
                            "{name}  ·  {}",
                            app.tf("o.questVariant", &[("v", v.to_string())])
                        );
                    }
                    (x.id.clone(), name)
                })
                .filter(|(id, name)| {
                    q.is_empty()
                        || name.to_lowercase().contains(&q)
                        || id.to_lowercase().contains(&q)
                })
                .collect();
            if list.is_empty() {
                note(ui, &app.t("c.nothingFound"));
            }
            egui::ScrollArea::vertical()
                .id_salt(("quest_pick_list", salt))
                .min_scrolled_height(240.0)
                .max_height(380.0)
                .show(ui, |ui| {
                    for (id, name) in list {
                        let resp = ui.add(
                            egui::Button::selectable(
                                id == current,
                                RichText::new(name).color(c.text),
                            )
                            .wrap(),
                        );
                        let resp = if expert {
                            resp.on_hover_text(id.clone())
                        } else {
                            resp
                        };
                        if resp.clicked() && id != current {
                            picked = Some(id);
                            ui.close();
                        }
                    }
                });
        });
    app.bufs.insert(key, query);
    picked
}

fn quest_list(app: &mut App, ui: &mut egui::Ui, r: &Value) {
    if r.at(&p(&["completedQuestDataManager", "completedQuests"]))
        .is_none()
    {
        return;
    }
    let done: Vec<String> = vault::arr(r, &["completedQuestDataManager", "completedQuests"])
        .iter()
        .filter_map(Value::as_text)
        .collect();
    let mut all: Vec<String> = catalog().quests.iter().map(|q| q.id.clone()).collect();
    for d in &done {
        if !all.contains(d) {
            all.push(d.clone());
        }
    }
    let mut toggle = None;
    panel(ui, |ui| {
        title(
            ui,
            &app.tf(
                "o.completedQuests",
                &[
                    ("n", done.len().to_string()),
                    ("all", catalog().quests.len().to_string()),
                ],
            ),
        );
        note(ui, &app.t("o.completedQuestsHelp"));
        let mut query = app.bufs.get("quest_query").cloned().unwrap_or_default();
        let mut filter = app
            .bufs
            .get("quest_filter")
            .cloned()
            .unwrap_or_else(|| "done".into());
        ui.horizontal_wrapped(|ui| {
            ui.label(icons::SEARCH);
            if ui
                .add(
                    egui::TextEdit::singleline(&mut query)
                        .hint_text(app.t("c.search"))
                        .desired_width(260.0),
                )
                .changed()
            {
                app.bufs.insert("quest_query".into(), query.clone());
            }
            for (v, k) in [
                ("done", "o.filter.done"),
                ("todo", "o.filter.todo"),
                ("all", "o.filter.all"),
            ] {
                if ui.selectable_label(filter == v, app.t(k)).clicked() {
                    filter = v.to_string();
                    app.bufs.insert("quest_filter".into(), filter.clone());
                }
            }
        });
        let q = query.to_lowercase();
        let rows: Vec<(String, String, bool)> = all
            .iter()
            .map(|id| (id.clone(), quest_line(app, id), done.contains(id)))
            .filter(|(id, name, is_done)| {
                let ok = match filter.as_str() {
                    "done" => *is_done,
                    "todo" => !*is_done,
                    _ => true,
                };
                ok && (q.is_empty()
                    || name.to_lowercase().contains(&q)
                    || id.to_lowercase().contains(&q))
            })
            .collect();
        ui.add_space(4.0);
        if rows.is_empty() {
            note(ui, &app.t("c.nothingFound"));
        }
        let tip = app.t("o.questToggleHelp");
        let expert = app.expert();
        egui::ScrollArea::vertical()
            .id_salt("quest_rows")
            .min_scrolled_height(260.0)
            .max_height(420.0)
            .show_rows(ui, 26.0, rows.len(), |ui, range| {
                for (id, name, is_done) in &rows[range] {
                    let mut v = *is_done;
                    let resp = ui.checkbox(&mut v, name).on_hover_text(if expert {
                        id.clone()
                    } else {
                        tip.clone()
                    });
                    if resp.changed() {
                        toggle = Some((id.clone(), v));
                    }
                }
            });
    });
    if let Some((id, done)) = toggle {
        app.act(Action::MarkQuest { id, done });
    }
}

fn wasteland(app: &mut App, ui: &mut egui::Ui) {
    let r = root(app);
    let teams = vault::arr(&r, &["vault", "wasteland", "teams"]);
    panel(ui, |ui| {
        title(ui, &app.t("o.explorers"));
        note(ui, &app.t("o.explorersHelp"));
        if teams.is_empty() {
            ui.add_space(4.0);
            note(ui, &app.t("o.noExplorers"));
        }
    });
    for (i, t) in teams.iter().enumerate() {
        ui.add_space(8.0);
        panel(ui, |ui| {
            let names: Vec<String> = t
                .get("dwellers")
                .and_then(Value::as_array)
                .map(|a| {
                    a.iter()
                        .filter_map(Value::as_i64)
                        .map(|d| dweller_name_any(&r, d))
                        .collect()
                })
                .unwrap_or_default();
            title(ui, &names.join(", "));
            let status = vault::text(t, &["status"]).unwrap_or_default();
            let key = format!("o.team.{status}");
            let status_text = match app.t(&key) {
                s if s == key => status.clone(),
                s => s,
            };
            info(ui, &app.t("o.status"), &status_text);
            if vault::team_dead(&r, t) {
                let ids: Vec<i64> = vault::arr(t, &["dwellers"])
                    .iter()
                    .filter_map(Value::as_i64)
                    .collect();
                ui.horizontal_wrapped(|ui| {
                    for (key, help, home) in [
                        ("o.reviveTeam", "dw.reviveTeamHelp", false),
                        ("dw.reviveHome", "dw.reviveHomeHelp", true),
                    ] {
                        if ui
                            .button(format!("{}  {}", icons::SPARKLES, app.t(key)))
                            .on_hover_text(app.t(help))
                            .clicked()
                        {
                            app.act(Action::Revive {
                                ids: ids.clone(),
                                home,
                            });
                        }
                    }
                });
            }
            let base = pi(&["vault", "wasteland", "teams"], i, &[]);
            let num = |k: &str| t.get(k).and_then(Value::as_f64);
            if let Some(s) = num("elapsedTimeAliveExploring") {
                if matches!(status.as_str(), "Exploring" | "GoingToQuest") {
                    let path = join(&base, "elapsedTimeAliveExploring");
                    let (l, h) = (app.t("o.exploringTime"), app.t("o.exploringTimeHelp"));
                    duration_row(app, ui, &path, &l, &h, s, 3600.0, |v| Action::SetInt {
                        path: path.clone(),
                        value: v as i64,
                    });
                } else {
                    info(ui, &app.t("o.exploring"), &duration(app, s));
                }
            }
            if status == "ReturningToVault" {
                if let (Some(total), Some(done)) =
                    (num("returnTripDuration"), num("elapsedReturningTime"))
                {
                    let path = join(&base, "elapsedReturningTime");
                    let left = (total - done).max(0.0);
                    let (l, h) = (app.t("o.returnIn"), app.t("o.returnInHelp"));
                    duration_row(app, ui, &path, &l, &h, left, 60.0, |v| Action::SetInt {
                        path: path.clone(),
                        value: (total - v).max(0.0) as i64,
                    });
                    if ui
                        .button(format!("{}  {}", icons::SPARKLES, app.t("o.returnNow")))
                        .on_hover_text(app.t("o.returnNowHelp"))
                        .clicked()
                    {
                        app.act(Action::SetInt {
                            path,
                            value: total.ceil().max(1.0) as i64,
                        });
                    }
                }
            }
            for (res, label, help, max) in [
                (
                    "Nuka",
                    "o.capsFound",
                    "o.capsFoundHelp",
                    shelter_core::rules::RESOURCE_MAX as f64,
                ),
                ("StimPack", "o.teamStim", "o.teamStimHelp", 25.0),
                ("RadAway", "o.teamRad", "o.teamRadHelp", 25.0),
            ] {
                let path = pi(
                    &["vault", "wasteland", "teams"],
                    i,
                    &["teamEquipment", "storage", "resources", res],
                );
                if r.at(&path).is_some() {
                    let o = Opts::text(app.t(label)).help(app.t(help)).range(0.0, max);
                    widgets::num_field(app, ui, &path, o);
                }
            }
            team_loot(app, ui, i, t);
            team_details(app, ui, i, t, &r);
        });
    }
}

/// Robots in the team, what everyone took along, and the team journal.
fn team_details(app: &mut App, ui: &mut egui::Ui, i: usize, t: &Value, r: &Value) {
    let c = style::colors(ui.ctx());
    let ru = app.settings.russian_item_names;
    let robots: Vec<String> = vault::arr(t, &["actors"])
        .iter()
        .filter_map(Value::as_i64)
        .map(|id| {
            vault::arr(r, &vault::ACTORS)
                .iter()
                .find(|a| vault::int(a, &["serializeId"]) == Some(id))
                .and_then(|a| vault::text(a, &["name"]))
                .filter(|n| !n.is_empty())
                .unwrap_or_else(|| app.t("dw.mrHandy"))
        })
        .collect();
    if !robots.is_empty() {
        info(ui, &app.t("o.teamRobots"), &robots.join(", "));
    }
    let initial = vault::arr(t, &["initialItems"]);
    let dwellers: Vec<i64> = vault::arr(t, &["dwellers"])
        .iter()
        .filter_map(Value::as_i64)
        .collect();
    if !initial.is_empty() {
        egui::CollapsingHeader::new(app.t("o.teamStart"))
            .id_salt(("team_start", i))
            .show(ui, |ui| {
                for (k, it) in initial.iter().enumerate() {
                    let who = dwellers
                        .get(k)
                        .map_or_else(String::new, |d| dweller_name_any(r, *d));
                    let name = |kind: ItemKind, key: &str| {
                        vault::text(it, &[key]).filter(|s| !s.is_empty()).map(|id| {
                            catalog()
                                .item(kind, &id)
                                .map_or(id.clone(), |x| x.name.get(ru).to_string())
                        })
                    };
                    let parts: Vec<String> = [
                        name(ItemKind::Weapon, "initWeapon"),
                        name(ItemKind::Outfit, "initOutfit"),
                        name(ItemKind::Pet, "initPet"),
                    ]
                    .into_iter()
                    .flatten()
                    .collect();
                    ui.add(egui::Label::new(format!("{who}: {}", parts.join(", "))).wrap());
                }
            });
    }
    if let Some(perf) = t.get("questOverallPerformance").and_then(Value::as_object) {
        let rows: Vec<(String, i64)> = perf
            .0
            .iter()
            .filter_map(|(k, v)| Some((k.text(), v.as_i64().filter(|n| *n > 0)?)))
            .collect();
        if !rows.is_empty() {
            egui::CollapsingHeader::new(app.t("o.teamResults"))
                .id_salt(("team_results", i))
                .show(ui, |ui| {
                    for (k, n) in rows {
                        let label = shelter_core::fields::describe_key(
                            app.lang,
                            Some("questOverallPerformance"),
                            &k,
                        )
                        .label;
                        info(ui, &label, &n.to_string());
                    }
                });
        }
    }
    let logs = vault::arr(t, &["logs"]);
    if !logs.is_empty() {
        egui::CollapsingHeader::new(app.tf("o.teamLog", &[("n", logs.len().to_string())]))
            .id_salt(("team_log", i))
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt(("team_log_rows", i))
                    .min_scrolled_height(200.0)
                    .max_height(320.0)
                    .show(ui, |ui| {
                        if !app.art.texts() {
                            let note = match app.art.status() {
                                crate::art::Status::Loading | crate::art::Status::Ready => {
                                    app.t("art.loading")
                                }
                                crate::art::Status::NoGame => app.t("o.logNoGame"),
                                _ => app.t("art.mismatch"),
                            };
                            ui.add(
                                egui::Label::new(RichText::new(note).small().color(c.muted)).wrap(),
                            );
                        }
                        for code in logs.iter().rev().filter_map(Value::as_text) {
                            let (time, text) = log_line(app, &code);
                            ui.horizontal_wrapped(|ui| {
                                ui.label(
                                    RichText::new(time).font(style::mono(12.0)).color(c.muted),
                                );
                                ui.add(egui::Label::new(text).wrap());
                            });
                        }
                    });
            });
    }
}

fn log_line(app: &App, code: &str) -> (String, String) {
    app.tx().log_line(code)
}

/// Incident block of the room editor: what is going on in this room, or a
/// way to start a fire or a creature attack here.
pub fn room_incident(app: &mut App, ui: &mut egui::Ui, root: &Value, room: &Value) {
    use shelter_core::actions::{room_max_health, NO_INCIDENT_ROOMS, ROOM_INCIDENTS};
    let c = style::colors(ui.ctx());
    let Some(id) = vault::int(room, &["deserializeID"]) else {
        return;
    };
    let kind = vault::text(room, &["type"]).unwrap_or_default();
    let Some(max) = room_max_health(room) else {
        return;
    };
    if NO_INCIDENT_ROOMS.contains(&kind.as_str()) {
        return;
    }
    ui.add_space(6.0);
    widgets::hint_label(ui, &app.t("room.incident"), &app.t("o.incidentStartHelp"));
    let burning = vault::text(room, &["currentStateName"]).as_deref() == Some("RoomEmergency");
    if burning {
        let mode = vault::int(room, &["currentState", "mode"]).unwrap_or(-1);
        ui.label(
            RichText::new(format!("{}  {}", icons::ALERT, incident_name(app, mode)))
                .color(c.danger),
        );
        if let Some(dmg) = room
            .at(&p(&["roomHealth", "damageValue"]))
            .and_then(Value::as_f64)
            .filter(|_| ROOM_INCIDENTS.contains(&mode))
        {
            let pct = (dmg / max * 100.0).round().clamp(1.0, 100.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(app.t("o.incidentStrength"));
                let key = format!("room_inc_strength:{id}");
                if let Some(v) =
                    widgets::pick_number(app, ui, &key, pct, (1.0, 100.0), Some((1.0, 100.0)), true)
                {
                    if v != pct {
                        app.act(Action::SetIncidentStrength {
                            room: Some(id),
                            value: v / 100.0,
                        });
                    }
                }
                widgets::unit_label(ui, "%");
            });
        }
        if ui
            .button(format!("{}  {}", icons::CHECK, app.t("o.incidentFinish")))
            .on_hover_text(app.t("o.incidentFinishHelp"))
            .clicked()
        {
            app.act(Action::FinishEmergency);
        }
        return;
    }
    if vault::flag(root, &["vault", "emergencyData", "active"]).unwrap_or(false) {
        ui.label(
            RichText::new(app.t("room.incidentBusy"))
                .small()
                .color(c.muted),
        );
        return;
    }
    let kind_key = format!("room_inc_kind:{id}");
    let str_key = format!("room_inc_str:{id}");
    let mut kind_sel: i64 = app
        .bufs
        .get(&kind_key)
        .and_then(|v| v.parse().ok())
        .unwrap_or(ROOM_INCIDENTS[0]);
    let mut str_sel: f64 = app
        .bufs
        .get(&str_key)
        .and_then(|v| v.parse().ok())
        .unwrap_or(45.0);
    ui.horizontal_wrapped(|ui| {
        for m in ROOM_INCIDENTS {
            if ui
                .selectable_label(kind_sel == m, incident_name(app, m))
                .clicked()
            {
                kind_sel = m;
            }
        }
    });
    ui.horizontal_wrapped(|ui| {
        ui.label(app.t("o.incidentStrength"))
            .on_hover_text(app.t("o.incidentStrengthHelp"));
        if let Some(v) = widgets::pick_number(
            app,
            ui,
            &format!("room_inc_new:{id}"),
            str_sel,
            (10.0, 100.0),
            Some((10.0, 100.0)),
            true,
        ) {
            str_sel = v;
        }
        widgets::unit_label(ui, "%");
    });
    if ui
        .button(format!(
            "{}  {}",
            icons::ALERT,
            app.t("o.incidentStartButton")
        ))
        .on_hover_text(app.t("o.incidentStartButtonHelp"))
        .clicked()
    {
        app.act(Action::StartIncident {
            room: id,
            kind: kind_sel,
            strength: str_sel / 100.0,
        });
    }
    app.bufs.insert(kind_key, kind_sel.to_string());
    app.bufs.insert(str_key, str_sel.to_string());
}

fn incident_name(app: &App, mode: i64) -> String {
    app.tx().incident_name(mode)
}

/// Current incident, its strength, starting a new one and the random check timer.
fn incidents(app: &mut App, ui: &mut egui::Ui, r: &Value) {
    use shelter_core::actions::{room_max_health, NO_INCIDENT_ROOMS, ROOM_INCIDENTS};
    let c = style::colors(ui.ctx());
    let em = p(&["vault", "emergencyData"]);
    title(ui, &app.t("o.incidents"));
    note(ui, &app.t("o.incidentsHelp"));
    let active = vault::flag(r, &["vault", "emergencyData", "active"]).unwrap_or(false);
    let rooms = vault::arr(r, &vault::ROOMS);
    let burning: Vec<&Value> = rooms
        .iter()
        .filter(|x| vault::text(x, &["currentStateName"]).as_deref() == Some("RoomEmergency"))
        .collect();
    let status = if active {
        let mode = vault::int(r, &["vault", "emergencyData", "mode"]).unwrap_or(-1);
        app.tf("o.incidentNow", &[("kind", incident_name(app, mode))])
    } else if burning.is_empty() {
        app.t("o.incidentNone")
    } else {
        app.t("o.incidentRooms")
    };
    info(ui, &app.t("o.status"), &status);
    for room in &burning {
        let Some(id) = vault::int(room, &["deserializeID"]) else {
            continue;
        };
        let mode = vault::int(room, &["currentState", "mode"]).unwrap_or(-1);
        let name = crate::sec_vault::room_title(app, room);
        let damage = room
            .at(&p(&["roomHealth", "damageValue"]))
            .and_then(Value::as_f64);
        ui.add_space(4.0);
        ui.add(
            egui::Label::new(
                RichText::new(format!(
                    "{}  {name}  ·  {}",
                    icons::ALERT,
                    incident_name(app, mode)
                ))
                .color(c.danger),
            )
            .wrap(),
        );
        if let (Some(dmg), Some(max), true) = (
            damage,
            room_max_health(room),
            ROOM_INCIDENTS.contains(&mode),
        ) {
            let pct = (dmg / max * 100.0).round().clamp(1.0, 100.0);
            widgets::hint_label(
                ui,
                &app.t("o.incidentStrength"),
                &app.t("o.incidentStrengthHelp"),
            );
            ui.horizontal_wrapped(|ui| {
                let key = format!("inc_strength:{id}");
                if let Some(v) =
                    widgets::pick_number(app, ui, &key, pct, (1.0, 100.0), Some((1.0, 100.0)), true)
                {
                    if v != pct {
                        app.act(Action::SetIncidentStrength {
                            room: Some(id),
                            value: v / 100.0,
                        });
                    }
                }
                widgets::unit_label(ui, "%");
            });
        }
    }
    let raider = join(&em, "raider");
    if active && r.at(&raider).is_some() {
        let life = r.at(&join(&raider, "life")).and_then(Value::as_f64);
        let max = r.at(&join(&raider, "lifeMax")).and_then(Value::as_f64);
        if let (Some(life), Some(max)) = (life, max) {
            widgets::hint_label(ui, &app.t("o.raidersHealth"), &app.t("o.raidersHealthHelp"));
            ui.horizontal_wrapped(|ui| {
                let cur = life.round().clamp(1.0, max.max(1.0));
                if let Some(v) = widgets::pick_number(
                    app,
                    ui,
                    "inc_raiders",
                    cur,
                    (1.0, max.max(1.0).round()),
                    Some((1.0, max.max(1.0).round())),
                    true,
                ) {
                    if v != cur {
                        app.act(Action::SetIncidentStrength {
                            room: None,
                            value: v,
                        });
                    }
                }
                widgets::unit_label(
                    ui,
                    &app.tf("o.ofMax", &[("n", widgets::fmt_num(max.round()))]),
                );
            });
        }
    } else if active && burning.is_empty() {
        note(ui, &app.t("o.incidentDoorNote"));
    }
    if active || !burning.is_empty() {
        if ui
            .button(format!("{}  {}", icons::CHECK, app.t("o.incidentFinish")))
            .on_hover_text(app.t("o.incidentFinishHelp"))
            .clicked()
        {
            app.act(Action::FinishEmergency);
        }
    } else {
        ui.add_space(6.0);
        widgets::hint_label(ui, &app.t("o.incidentStart"), &app.t("o.incidentStartHelp"));
        let pick_room: i64 = app
            .bufs
            .get("inc_room")
            .and_then(|v| v.parse().ok())
            .unwrap_or(-1);
        let pick_kind: i64 = app
            .bufs
            .get("inc_kind")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        let strength: f64 = app
            .bufs
            .get("inc_strength")
            .and_then(|v| v.parse().ok())
            .unwrap_or(45.0);
        let candidates: Vec<(i64, String)> = rooms
            .iter()
            .filter(|x| {
                let k = vault::text(x, &["type"]).unwrap_or_default();
                !NO_INCIDENT_ROOMS.contains(&k.as_str()) && room_max_health(x).is_some()
            })
            .filter_map(|x| {
                Some((
                    vault::int(x, &["deserializeID"])?,
                    crate::sec_vault::room_title(app, x),
                ))
            })
            .collect();
        let mut room_sel = pick_room;
        let mut kind_sel = pick_kind;
        let mut str_sel = strength;
        let width = ui.available_width().min(300.0);
        egui::ComboBox::from_id_salt("inc_room")
            .selected_text(
                candidates
                    .iter()
                    .find(|(id, _)| *id == room_sel)
                    .map_or_else(|| app.t("o.incidentPickRoom"), |(_, n)| n.clone()),
            )
            .width(width)
            .show_ui(ui, |ui| {
                for (id, name) in &candidates {
                    if ui.selectable_label(*id == room_sel, name).clicked() {
                        room_sel = *id;
                    }
                }
            });
        ui.horizontal_wrapped(|ui| {
            for m in ROOM_INCIDENTS {
                if ui
                    .selectable_label(kind_sel == m, incident_name(app, m))
                    .clicked()
                {
                    kind_sel = m;
                }
            }
        });
        widgets::hint_label(
            ui,
            &app.t("o.incidentStrength"),
            &app.t("o.incidentStrengthHelp"),
        );
        ui.horizontal_wrapped(|ui| {
            if let Some(v) = widgets::pick_number(
                app,
                ui,
                "inc_new_strength",
                str_sel,
                (10.0, 100.0),
                Some((10.0, 100.0)),
                true,
            ) {
                str_sel = v;
            }
            widgets::unit_label(ui, "%");
        });
        if ui
            .add_enabled(
                room_sel >= 0,
                egui::Button::new(format!(
                    "{}  {}",
                    icons::ALERT,
                    app.t("o.incidentStartButton")
                )),
            )
            .on_hover_text(app.t("o.incidentStartButtonHelp"))
            .clicked()
        {
            app.act(Action::StartIncident {
                room: room_sel,
                kind: kind_sel,
                strength: str_sel / 100.0,
            });
        }
        app.bufs.insert("inc_room".into(), room_sel.to_string());
        app.bufs.insert("inc_kind".into(), kind_sel.to_string());
        app.bufs.insert("inc_strength".into(), str_sel.to_string());
    }
    if let Some(task) = vault::int(r, &["vault", "emergencyData", "randomEventTaskId"]) {
        if task >= 0 {
            ui.add_space(6.0);
            let (l, h) = (
                app.t("o.incidentNextCheck"),
                app.t("o.incidentNextCheckHelp"),
            );
            task_timer(app, ui, task, &l, &h);
            if let Some(left) = task_left(r, task) {
                ui.horizontal_wrapped(|ui| {
                    ui.label(app.t("o.incidentPause"))
                        .on_hover_text(app.t("o.incidentPauseHelp"));
                    for (hours, key) in
                        [(1.0, "o.pause1h"), (6.0, "o.pause6h"), (24.0, "o.pause24h")]
                    {
                        if ui.button(app.t(key)).clicked() {
                            app.act(Action::SetTaskTime {
                                id: task,
                                seconds: left + hours * 3600.0,
                            });
                        }
                    }
                });
            }
        }
    }
    season_waves(app, ui, r);
}

/// Seconds left on a task of the task manager.
fn task_left(r: &Value, id: i64) -> Option<f64> {
    let now = r.at(&p(&["taskMgr", "time"])).and_then(Value::as_f64)?;
    let t = vault::arr(r, &["taskMgr", "tasks"])
        .iter()
        .find(|t| vault::int(t, &["id"]) == Some(id))?;
    Some((t.get("endTime").and_then(Value::as_f64)? - now).max(0.0))
}

/// Scorched raids of the ultracite season; only season vaults use them.
fn season_waves(app: &mut App, ui: &mut egui::Ui, r: &Value) {
    let season = vault::text(r, &["seasonId"]).is_some_and(|s| !s.is_empty());
    let em = p(&["vault", "emergencyData"]);
    if !season || r.at(&join(&em, "currentWave")).is_none() {
        return;
    }
    ui.add_space(8.0);
    widgets::hint_label(ui, &app.t("o.waves"), &app.t("o.wavesHelp"));
    let o = Opts::text(app.t("o.waveNumber")).range(0.0, 100.0);
    widgets::num_field(app, ui, &join(&em, "currentWave"), o);
    let failed = join(&em, "failedPreviousWave");
    if r.at(&failed).is_some() {
        let o = Opts::text(app.t("o.waveFailed")).help(app.t("o.waveFailedHelp"));
        widgets::bool_field(app, ui, &failed, o);
    }
    for (k, label) in [
        ("currentWaveRemainingMelee", "o.waveMelee"),
        ("currentWaveRemainingGunner", "o.waveGunner"),
        ("currentWaveRemainingHeavy", "o.waveHeavy"),
    ] {
        let path = join(&em, k);
        if r.at(&path).is_some() {
            let o = Opts::text(app.t(label)).range(0.0, 50.0);
            widgets::num_field(app, ui, &path, o);
        }
    }
}

/// Items and theme parts a wasteland team has found.
fn team_loot(app: &mut App, ui: &mut egui::Ui, team: usize, t: &Value) {
    let c = style::colors(ui.ctx());
    let items: Vec<Value> = t
        .at(&p(&["teamEquipment", "inventory", "items"]))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let load = shelter_core::actions::team_load(t);
    let max = catalog().limits.team_items_max;
    ui.add_space(6.0);
    widgets::hint_label(
        ui,
        &app.tf(
            "o.loot",
            &[("n", load.to_string()), ("max", max.to_string())],
        ),
        &app.t("o.lootHelp"),
    );
    // Same item, type and recipe flag form one line.
    let mut groups: Vec<(String, String, bool, Vec<usize>)> = Vec::new();
    for (i, it) in items.iter().enumerate() {
        let id = vault::text(it, &["id"]).unwrap_or_default();
        let kind = vault::text(it, &["type"]).unwrap_or_default();
        let recipe = vault::flag(it, &["extraData", "isRecipe"]).unwrap_or(false);
        match groups
            .iter_mut()
            .find(|g| g.0 == id && g.1 == kind && g.2 == recipe)
        {
            Some(g) => g.3.push(i),
            None => groups.push((id, kind, recipe, vec![i])),
        }
    }
    let mut remove: Option<Vec<usize>> = None;
    if groups.is_empty() {
        note(ui, &app.t("o.lootEmpty"));
    }
    let lines: Vec<(String, String)> = groups
        .iter()
        .map(|(id, kind, recipe, _)| {
            let name = ItemKind::parse(kind)
                .and_then(|k| catalog().item(k, id))
                .map_or_else(
                    || id.clone(),
                    |i| i.name.get(app.settings.russian_item_names).to_string(),
                );
            let mut kind_text = app.t(&format!("it.kind.{kind}"));
            if *recipe {
                kind_text = format!("{kind_text}  ·  {}", app.t("o.lootRecipe"));
            }
            (name, kind_text)
        })
        .collect();
    let small = egui::TextStyle::Small.resolve(ui.style());
    let width = |f: &dyn Fn(usize) -> f32| (0..groups.len()).map(f).fold(0.0_f32, f32::max) + 2.0;
    let name_w =
        width(&|i| widgets::text_width(ui, &lines[i].0, egui::TextStyle::Body.resolve(ui.style())))
            .min((ui.available_width() * 0.4).max(80.0));
    let kind_w = width(&|i| widgets::text_width(ui, &lines[i].1, small.clone()));
    let count_w =
        width(&|i| widgets::text_width(ui, &format!("× {}", groups[i].3.len()), style::mono(13.0)));
    for ((id, kind, _, idx), (name, kind_text)) in groups.iter().zip(&lines) {
        ui.horizontal(|ui| {
            widgets::cell(ui, name_w, egui::Align::Min, |ui| {
                let resp = ui.add(
                    egui::Label::new(name)
                        .truncate()
                        .sense(egui::Sense::hover()),
                );
                if let Some(k) = ItemKind::parse(kind) {
                    crate::sec_vault::item_hover(app, resp, k, id);
                }
            });
            widgets::cell(ui, kind_w, egui::Align::Min, |ui| {
                ui.label(RichText::new(kind_text).small().color(c.muted));
            });
            widgets::cell(ui, count_w, egui::Align::Max, |ui| {
                ui.label(RichText::new(format!("× {}", idx.len())).font(style::mono(13.0)));
            });
            if crate::app::small_icon(ui, icons::X, &app.t("o.lootRemoveOne")).clicked() {
                remove = idx.last().map(|i| vec![*i]);
            }
            if idx.len() > 1
                && crate::app::small_icon(ui, icons::TRASH, &app.t("o.lootRemoveAll")).clicked()
            {
                remove = Some(idx.clone());
            }
        });
    }
    if let Some(indices) = remove {
        app.act(Action::RemoveTeamItems { team, indices });
    }
    let qkey = format!("team_loot_q:{team}");
    let rkey = format!("team_loot_r:{team}");
    let mut q = app.bufs.remove(&qkey).unwrap_or_default();
    let mut as_recipe = app.bufs.get(&rkey).is_some_and(|v| v == "1");
    ui.horizontal_wrapped(|ui| {
        let picked = crate::sec_vault::item_picker(
            app,
            ui,
            &format!("team_add_{team}"),
            &[ItemKind::Weapon, ItemKind::Outfit, ItemKind::Junk],
            &mut q,
        );
        ui.checkbox(&mut as_recipe, app.t("o.lootAsRecipe"))
            .on_hover_text(app.t("o.lootAsRecipeHelp"));
        if let Some((kind, id)) = picked {
            let recipe = as_recipe && kind != ItemKind::Junk;
            if app.act(Action::AddTeamItems {
                team,
                kind,
                id,
                count: 1,
                recipe,
            }) {
                q.clear();
            }
        }
    });
    app.bufs.insert(qkey, q);
    app.bufs
        .insert(rkey, if as_recipe { "1" } else { "0" }.to_string());
    let themes: Vec<(String, i64)> = t
        .at(&p(&["teamEquipment", "collectedThemes", "themeList"]))
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|th| {
                    Some((
                        vault::text(th, &["id"])?,
                        vault::int(th, &["extraData", "partsCollectedCount"]).unwrap_or(0),
                    ))
                })
                .collect()
        })
        .unwrap_or_default();
    if t.at(&p(&["teamEquipment", "collectedThemes", "themeList"]))
        .is_none()
    {
        return;
    }
    ui.add_space(6.0);
    widgets::hint_label(ui, &app.t("o.lootThemes"), &app.t("o.lootThemesHelp"));
    let ru = app.lang == shelter_core::i18n::Lang::Ru;
    let tname = |id: &str| {
        catalog()
            .theme(id)
            .map_or_else(|| id.to_string(), |t| t.name.get(ru).to_string())
    };
    let theme_w = themes
        .iter()
        .map(|(id, _)| {
            widgets::text_width(ui, &tname(id), egui::TextStyle::Body.resolve(ui.style()))
        })
        .fold(0.0_f32, f32::max)
        .min((ui.available_width() * 0.4).max(80.0))
        + 2.0;
    for (id, parts) in &themes {
        ui.horizontal(|ui| {
            widgets::cell(ui, theme_w, egui::Align::Min, |ui| {
                let r = ui.add(
                    egui::Label::new(tname(id))
                        .truncate()
                        .sense(egui::Sense::hover()),
                );
                crate::sec_vault::theme_hover(app, r, id);
            });
            let key = format!("team_theme:{team}:{id}");
            if let Some(v) =
                widgets::pick_number(app, ui, &key, *parts as f64, (0.0, 9.0), None, true)
            {
                if v as i64 != *parts {
                    app.act(Action::SetTeamThemeParts {
                        team,
                        id: id.clone(),
                        parts: v as i64,
                    });
                }
            }
            widgets::unit_label(ui, &app.t("theme.partsOf"));
        });
    }
    let mut add = None;
    egui::ComboBox::from_id_salt(("team_theme_add", team))
        .selected_text(RichText::new(app.t("o.lootThemeAdd")).color(c.muted))
        .width(ui.available_width().min(280.0))
        .show_ui(ui, |ui| {
            for th in catalog().themes.iter().filter(|th| th.recipe) {
                if themes.iter().any(|(id, _)| *id == th.id) {
                    continue;
                }
                let r = ui.selectable_label(false, tname(&th.id));
                if crate::sec_vault::theme_hover(app, r, &th.id).clicked() {
                    add = Some(th.id.clone());
                }
            }
        });
    if let Some(id) = add {
        app.act(Action::SetTeamThemeParts { team, id, parts: 1 });
    }
}

fn objective_params(app: &App, o: &Objective, req: Option<&Value>, goal: i64) -> Vec<String> {
    app.tx().objective_params(o, req, goal)
}

fn objective_text(app: &App, id: &str, level: i64, req: Option<&Value>) -> String {
    app.tx().objective_text(id, level, req)
}

fn reward_text(app: &App, o: &Objective, level: i64) -> String {
    app.tx().reward_text(o, level)
}

/// Timer row for a game task: minutes until it fires, editable.
pub(crate) fn task_timer(app: &mut App, ui: &mut egui::Ui, id: i64, label: &str, help: &str) {
    let r = root(app);
    let now = r.at(&p(&["taskMgr", "time"])).and_then(Value::as_f64);
    let tasks = vault::arr(&r, &["taskMgr", "tasks"]);
    let Some((i, t)) = tasks
        .iter()
        .enumerate()
        .find(|(_, t)| vault::int(t, &["id"]) == Some(id))
    else {
        return;
    };
    let (Some(now), Some(end)) = (now, t.get("endTime").and_then(Value::as_f64)) else {
        return;
    };
    let left = (end - now).max(0.0);
    let path = pi(&["taskMgr", "tasks"], i, &[]);
    minutes_row(app, ui, &path, label, help, left, |secs| {
        Action::SetTaskTime { id, seconds: secs }
    });
    if left > 2.0
        && ui
            .small_button(format!("{}  {}", icons::CHECK, app.t("o.timerNow")))
            .on_hover_text(app.t("o.timerNowHelp"))
            .clicked()
    {
        app.act(Action::SetTaskTime { id, seconds: 1.0 });
    }
}

/// Editable duration in minutes, with the readable time next to it.
fn minutes_row(
    app: &mut App,
    ui: &mut egui::Ui,
    path: &[Seg],
    label: &str,
    help: &str,
    secs: f64,
    make: impl FnOnce(f64) -> Action,
) {
    duration_row(app, ui, path, label, help, secs, 60.0, make);
}

/// Editable duration in `unit` seconds (60 = minutes), with the readable
/// time next to it when it is long.
#[allow(clippy::too_many_arguments)]
fn duration_row(
    app: &mut App,
    ui: &mut egui::Ui,
    path: &[Seg],
    label: &str,
    help: &str,
    secs: f64,
    unit: f64,
    make: impl FnOnce(f64) -> Action,
) {
    let n = (secs / unit).ceil();
    let (max, unit_key) = if unit >= 3600.0 {
        (30.0 * 24.0, "o.hours")
    } else if unit >= 60.0 {
        (24.0 * 60.0, "o.minutes")
    } else {
        (3600.0f64, "o.seconds")
    };
    let max = max.max(n);
    let key = format!("{}#duration", path_to_string(path));
    let mut tail = app.t(unit_key);
    if secs >= 3600.0 || unit >= 3600.0 || (unit < 60.0 && secs >= 60.0) {
        tail = format!("{tail} ({})", duration(app, secs));
    }
    let o = Opts {
        derived: true,
        ..Opts::text(label.to_string()).help(help.to_string())
    };
    let mut out = None;
    widgets::row(app, ui, path, &o, |app, ui| {
        if let Some(v) = widgets::pick_number(app, ui, &key, n, (0.0, max), Some((0.0, max)), true)
        {
            out = Some(v);
        }
        widgets::unit_label(ui, &tail);
    });
    if let Some(v) = out {
        app.act(make(v * unit));
    }
}

/// Fraction stored in the save, edited in percent.
fn percent_row(app: &mut App, ui: &mut egui::Ui, path: &[Seg], label: &str, help: &str) {
    let Some(v) = root(app).at(path).and_then(Value::as_f64) else {
        return;
    };
    let pct = (v * 100.0).round();
    let max = 100.0f64.max(pct);
    let key = format!("{}#pct", path_to_string(path));
    let o = Opts {
        derived: true,
        ..Opts::text(label.to_string()).help(help.to_string())
    };
    let mut out = None;
    widgets::row(app, ui, path, &o, |app, ui| {
        if let Some(n) =
            widgets::pick_number(app, ui, &key, pct, (0.0, max), Some((0.0, 100.0)), true)
        {
            out = Some(n);
        }
        widgets::unit_label(ui, "%");
    });
    if let Some(n) = out {
        app.act(Action::SetNumber {
            path: path.to_vec(),
            value: n / 100.0,
        });
    }
}

fn objectives(app: &mut App, ui: &mut egui::Ui) {
    let r = root(app);
    let ru = app.lang == Lang::Ru;
    let c = style::colors(ui.ctx());
    let slots = vault::arr(&r, &["objectiveMgr", "slotArray"]);
    let used: Vec<String> = slots
        .iter()
        .filter_map(|s| {
            s.at(&p(&["objective", "objectiveID"]))
                .and_then(Value::as_text)
        })
        .collect();
    let survival = vault::text(&r, &["vault", "VaultMode"]).as_deref() == Some("Survival");
    let expert = app.expert();
    let mut complete = None;
    let mut replace: Option<(usize, String, i64)> = None;
    panel(ui, |ui| {
        title(ui, &app.t("o.currentObjectives"));
        note(ui, &app.t("o.currentObjectivesHelp"));
        if slots.is_empty() {
            note(ui, &app.t("c.empty"));
        }
    });
    for (i, s) in slots.iter().enumerate() {
        let Some(id) = s
            .at(&p(&["objective", "objectiveID"]))
            .and_then(Value::as_text)
        else {
            continue;
        };
        let level = s
            .at(&p(&["objective", "incrementLevel"]))
            .and_then(Value::as_i64)
            .unwrap_or(0);
        let req = s
            .at(&p(&["objective", "requirements"]))
            .and_then(Value::as_array)
            .and_then(|a| a.first());
        let info = catalog().objective(&id);
        let done = s
            .at(&p(&["objective", "completed"]))
            .and_then(Value::as_bool)
            .unwrap_or(false);
        ui.add_space(8.0);
        panel(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(icons::FLAG).color(c.accent));
                ui.add(
                    egui::Label::new(
                        RichText::new(objective_text(app, &id, level, req)).font(style::bold(15.0)),
                    )
                    .wrap(),
                );
            });
            if let Some(o) = info {
                info_line(ui, &app.t("o.reward"), &reward_text(app, o, level));
            }
            if expert {
                info_line(ui, "ID", &id);
            }
            if done {
                ui.label(
                    RichText::new(format!("{}  {}", icons::CHECK, app.t("o.objectiveDone")))
                        .color(c.success),
                );
            }
            if let Some(o) = info {
                let max = o
                    .goal
                    .as_ref()
                    .and_then(|g| g.max_level)
                    .filter(|_| o.goal.as_ref().is_some_and(|g| g.step != 0));
                if let Some(max) = max.filter(|m| *m > 0) {
                    let path = p(&["objectiveMgr", "slotArray"]);
                    let mut path = path;
                    path.push(Seg::Index(i));
                    path.push(Seg::Key("objective".into()));
                    path.push(Seg::Key("incrementLevel".into()));
                    let opts = Opts::text(app.t("o.objLevel"))
                        .help(app.t("o.objLevelHelp"))
                        .range(0.0, max as f64);
                    widgets::num_field(app, ui, &path, opts);
                }
            }
            ui.add_space(2.0);
            ui.horizontal_wrapped(|ui| {
                if !done
                    && ui
                        .button(format!(
                            "{}  {}",
                            icons::CHECK,
                            app.t("o.completeObjective")
                        ))
                        .on_hover_text(app.t("o.completeObjectiveHelp"))
                        .clicked()
                {
                    complete = Some(i);
                }
                let query_key = format!("obj_query{i}");
                let mut query = app.bufs.get(&query_key).cloned().unwrap_or_default();
                let btn = ui
                    .button(format!(
                        "{}  {}",
                        icons::PENCIL,
                        app.t("o.replaceObjective")
                    ))
                    .on_hover_text(app.t("o.replaceObjectiveHelp"));
                egui::Popup::menu(&btn)
                    .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                    .show(|ui| {
                        ui.set_min_width(420.0);
                        ui.set_max_width(460.0);
                        ui.horizontal(|ui| {
                            ui.label(icons::SEARCH);
                            ui.add(
                                egui::TextEdit::singleline(&mut query)
                                    .hint_text(app.t("c.search"))
                                    .desired_width(360.0),
                            );
                        });
                        let q = query.to_lowercase();
                        let list: Vec<(&Objective, String, i64)> = catalog()
                            .objectives
                            .iter()
                            .filter(|o| {
                                expert
                                    || (!o.tutorial
                                        && !o.season
                                        && if survival { o.survival } else { o.normal })
                            })
                            .filter(|o| o.id == id || !used.contains(&o.id))
                            .map(|o| {
                                let lvl = o.level_used(level);
                                let params = objective_params(app, o, None, o.goal_value(lvl));
                                (o, o.text(ru, lvl, &params), lvl)
                            })
                            .filter(|(o, t, _)| {
                                q.is_empty()
                                    || t.to_lowercase().contains(&q)
                                    || o.id.to_lowercase().contains(&q)
                            })
                            .collect();
                        ui.add_space(4.0);
                        if list.is_empty() {
                            note(ui, &app.t("c.nothingFound"));
                        }
                        egui::ScrollArea::vertical()
                            .id_salt(("obj_list", i))
                            .min_scrolled_height(240.0)
                            .max_height(380.0)
                            .show(ui, |ui| {
                                for (o, text, lvl) in list {
                                    let mut job = egui::text::LayoutJob::default();
                                    job.append(
                                        &text,
                                        0.0,
                                        egui::TextFormat::simple(
                                            egui::FontId::proportional(14.0),
                                            c.text,
                                        ),
                                    );
                                    job.append(
                                        &format!(
                                            "\n{}  ·  {}",
                                            app.tf("o.objTier", &[("n", o.level.to_string())]),
                                            reward_text(app, o, lvl)
                                        ),
                                        0.0,
                                        egui::TextFormat::simple(
                                            egui::FontId::proportional(12.0),
                                            c.muted,
                                        ),
                                    );
                                    let resp =
                                        ui.add(egui::Button::selectable(o.id == id, job).wrap());
                                    let resp = if expert {
                                        resp.on_hover_text(o.id.clone())
                                    } else {
                                        resp
                                    };
                                    if resp.clicked() && o.id != id {
                                        replace = Some((i, o.id.clone(), lvl));
                                        ui.close();
                                    }
                                }
                            });
                    });
                app.bufs.insert(query_key, query);
            });
        });
    }
    if let Some(slot) = complete {
        app.act(Action::CompleteObjective { slot });
    }
    if let Some((slot, id, level)) = replace {
        app.act(Action::ReplaceObjective { slot, id, level });
    }
    let base = p(&["objectiveMgr"]);
    if r.at(&base).is_some() {
        ui.add_space(8.0);
        panel(ui, |ui| {
            title(ui, &app.t("o.objSettings"));
            let can = join(&base, "canDiscard");
            if r.at(&can).is_some() {
                let o = Opts::text(app.t("o.freeSkip")).help(app.t("o.freeSkipHelp"));
                widgets::bool_field(app, ui, &can, o);
            }
            let skips = join(&base, "nukaQuantumIncrement");
            if r.at(&skips).is_some() {
                let o = Opts::text(app.t("o.quantumSkips"))
                    .help(app.t("o.quantumSkipsHelp"))
                    .range(0.0, 10.0);
                widgets::num_field(app, ui, &skips, o);
            }
            if let Some(task) = r.at(&join(&base, "taskID")).and_then(Value::as_i64) {
                let (l, h) = (app.t("o.newObjectivesIn"), app.t("o.newObjectivesInHelp"));
                task_timer(app, ui, task, &l, &h);
            }
        });
    }
    ui.add_space(8.0);
    achievements(app, ui, &r);
}

fn info_line(ui: &mut egui::Ui, name: &str, value: &str) {
    info(ui, name, value);
}

fn achievements(app: &mut App, ui: &mut egui::Ui, r: &Value) {
    let ru = app.lang == Lang::Ru;
    let expert = app.expert();
    let done: Vec<String> = vault::arr(r, &["vault", "Achievements", "completed"])
        .iter()
        .filter_map(Value::as_text)
        .collect();
    if r.at(&p(&["vault", "Achievements"])).is_none() {
        return;
    }
    let list: Vec<_> = catalog()
        .achievements
        .iter()
        .filter(|a| expert || !a.steam_id.is_empty())
        .collect();
    let mut toggle = None;
    panel(ui, |ui| {
        let n = list
            .iter()
            .filter(|a| done.contains(&a.objective_id))
            .count();
        title(
            ui,
            &app.tf(
                "o.achievements",
                &[("n", n.to_string()), ("all", list.len().to_string())],
            ),
        );
        note(ui, &app.t("o.achievementsHelp"));
        ui.add_space(4.0);
        let names: Vec<String> = list
            .iter()
            .map(|a| {
                let key = format!("ach.{}", a.id);
                let name = app.t(&key);
                if name == key {
                    a.name.get(ru).to_string()
                } else {
                    name
                }
            })
            .collect();
        let sp = ui.spacing();
        let tick = sp.icon_width + sp.icon_spacing + sp.button_padding.x * 2.0;
        let body = egui::TextStyle::Body.resolve(ui.style());
        let name_w = (names
            .iter()
            .map(|n| widgets::text_width(ui, n, body.clone()))
            .fold(0.0_f32, f32::max)
            + tick
            + 4.0)
            .min((ui.available_width() * 0.6).max(120.0));
        for (a, name) in list.iter().zip(names) {
            let mut v = done.contains(&a.objective_id);
            ui.horizontal(|ui| {
                let resp =
                    widgets::cell(ui, name_w, egui::Align::Min, |ui| ui.checkbox(&mut v, name));
                let resp = if expert {
                    resp.on_hover_text(format!("{} / {}", a.objective_id, a.steam_id))
                } else {
                    resp.on_hover_text(app.t("o.achievementToggleHelp"))
                };
                if resp.changed() {
                    toggle = Some((a.objective_id.clone(), v));
                }
                if !v {
                    achievement_progress(app, ui, r, &a.objective_id, a.goal);
                }
            });
        }
    });
    if let Some((id, done)) = toggle {
        app.act(Action::SetAchievement { id, done });
    }
}

/// Counter of an achievement in progress, with the goal when it is a count.
fn achievement_progress(app: &mut App, ui: &mut egui::Ui, r: &Value, id: &str, goal: i64) {
    let list = vault::arr(r, &["vault", "Achievements", "objectivesInProgress"]);
    let Some(i) = list
        .iter()
        .position(|o| vault::text(o, &["objectiveID"]).as_deref() == Some(id))
    else {
        return;
    };
    let Some(req) = vault::arr(&list[i], &["requirements"]).first() else {
        return;
    };
    let Some(obj) = req.as_object() else { return };
    // The first number is the counter; ids and flags are skipped.
    let Some((key, cur)) = obj.0.iter().find_map(|(k, v)| {
        let k = k.text();
        (!matches!(k.as_str(), "dweller" | "acceptedRoom" | "requirementID"))
            .then_some(())
            .and(v.as_f64().filter(|_| matches!(v, Value::Number(_))))
            .map(|n| (k, n))
    }) else {
        return;
    };
    let path = pi(
        &["vault", "Achievements", "objectivesInProgress"],
        i,
        &["requirements"],
    );
    let mut path = path;
    path.push(Seg::Index(0));
    path.push(Seg::Key(key.clone()));
    let max = if goal > 0 { goal as f64 } else { 1_000_000.0 };
    if let Some(v) = widgets::pick_number(
        app,
        ui,
        &format!("ach:{id}"),
        cur,
        (0.0, max.max(cur)),
        None,
        true,
    ) {
        if v != cur {
            app.act(Action::SetNumber { path, value: v });
        }
    }
    if goal > 0 {
        widgets::unit_label(ui, &app.tf("o.ofMax", &[("n", goal.to_string())]));
    }
}

fn stats(app: &mut App, ui: &mut egui::Ui) {
    let r = root(app);
    let base = p(&["StatsWindow", "vaultData"]);
    let Some(Value::Object(o)) = r.at(&base).cloned() else {
        panel(ui, |ui| note(ui, &app.t("c.empty")));
        return;
    };
    let hidden = [
        "previousVResources",
        "vaultCreatedBeforeUpdate",
        "weaponFactoryBuilt",
        "outfitFactoryBuilt",
        "measuringSince",
        "collectedRes",
    ];
    panel(ui, |ui| {
        title(ui, &app.t("o.stats"));
        note(ui, &app.t("o.statsHelp"));
        if let Some(Value::Number(_)) = o.get("measuringSince") {
            let path = join(&base, "measuringSince");
            let opts = Opts::at(app, &path);
            widgets::ticks_field(app, ui, &path, opts);
        }
        for (k, v) in &o.0 {
            let key = k.text();
            if hidden.contains(&key.as_str()) {
                continue;
            }
            let path = join(&base, &key);
            let opts = Opts::at(app, &path);
            match v {
                Value::Number(_) => widgets::num_field(app, ui, &path, opts),
                Value::Bool(_) => widgets::bool_field(app, ui, &path, opts),
                _ => {}
            }
        }
    });
    if let Some(Value::Object(res)) = o.get("collectedRes") {
        ui.add_space(8.0);
        panel(ui, |ui| {
            title(ui, &app.t("o.collectedRes"));
            note(ui, &app.t("o.collectedResHelp"));
            for (k, _) in &res.0 {
                let key = k.text();
                let label_key = format!("res.{key}");
                let label = app.t(&label_key);
                if label == label_key {
                    continue;
                }
                let path = join(&join(&base, "collectedRes"), &key);
                widgets::num_field(app, ui, &path, Opts::text(label));
            }
        });
    }
}

fn tutorial(app: &mut App, ui: &mut egui::Ui) {
    let r = root(app);
    let base = p(&["tutorialManager"]);
    let Some(Value::Object(o)) = r.at(&base).cloned() else {
        panel(ui, |ui| note(ui, &app.t("c.empty")));
        return;
    };
    panel(ui, |ui| {
        title(ui, &app.t("o.tutorialMain"));
        note(ui, &app.t("o.tutorialMainHelp"));
        ui.add_space(4.0);
        if ui
            .button(format!("{}  {}", icons::CHECK, app.t("o.completeTutorial")))
            .on_hover_text(app.t("o.completeTutorialHelp"))
            .clicked()
        {
            app.act(Action::CompleteTutorial);
        }
    });
    ui.add_space(8.0);
    let hints: Vec<String> =
        o.0.iter()
            .map(|(k, _)| k.text())
            .filter(|k| k.starts_with("Contextual"))
            .collect();
    panel(ui, |ui| {
        let shown = hints
            .iter()
            .filter(|k| o.get(k).and_then(Value::as_bool).unwrap_or(false))
            .count();
        title(
            ui,
            &app.tf(
                "o.hints",
                &[("n", shown.to_string()), ("all", hints.len().to_string())],
            ),
        );
        note(ui, &app.t("o.hintsHelp"));
        ui.horizontal_wrapped(|ui| {
            if ui.button(app.t("o.hintsAllShown")).clicked() {
                app.act(Action::SetHints { shown: true });
            }
            if ui.button(app.t("o.hintsAllNew")).clicked() {
                app.act(Action::SetHints { shown: false });
            }
        });
        ui.add_space(4.0);
        for k in &hints {
            let path = join(&base, k);
            let label = Opts::at(app, &path).label;
            let opts = Opts::text(label).help(app.t("o.hintHelp"));
            widgets::bool_field(app, ui, &path, opts);
        }
    });
}

fn events(app: &mut App, ui: &mut egui::Ui) {
    let r = root(app);
    panel(ui, |ui| {
        title(ui, &app.t("o.events"));
        note(ui, &app.t("o.eventsHelp"));
    });
    ui.add_space(8.0);
    let ms = p(&["MysteriousStranger"]);
    if r.at(&ms).is_some() {
        panel(ui, |ui| {
            title(ui, &app.t("o.stranger"));
            note(ui, &app.t("o.strangerHelp"));
            let can = join(&ms, "canAppear");
            if r.at(&can).is_some() {
                let o = Opts::text(app.t("o.strangerCan")).help(app.t("o.strangerCanHelp"));
                widgets::bool_field(app, ui, &can, o);
            }
            let hiding = vault::text(&r, &["MysteriousStranger", "currentState"]).as_deref()
                == Some("Hiding");
            let num = |k: &str| r.at(&join(&ms, k)).and_then(Value::as_f64);
            if hiding {
                if let Some(total) = num("timeToAppear") {
                    let left = (total - num("remainingTimeToAppear").unwrap_or(0.0)).max(0.0);
                    let (l, h) = (app.t("o.strangerIn"), app.t("o.strangerInHelp"));
                    duration_row(
                        app,
                        ui,
                        &join(&ms, "remainingTimeToAppear"),
                        &l,
                        &h,
                        left,
                        1.0,
                        |s| Action::SetStrangerTime { seconds: s },
                    );
                    if ui
                        .button(format!("{}  {}", icons::SPARKLES, app.t("o.strangerNow")))
                        .on_hover_text(app.t("o.strangerNowHelp"))
                        .clicked()
                    {
                        app.act(Action::SetStrangerTime { seconds: 10.0 });
                    }
                }
            } else {
                info(ui, &app.t("o.status"), &app.t("o.strangerHere"));
                if let Some(total) = num("timeToHide") {
                    let left = (total - num("remainingTimeToHide").unwrap_or(0.0)).max(0.0);
                    let (l, h) = (app.t("o.strangerLeaves"), app.t("o.strangerLeavesHelp"));
                    duration_row(
                        app,
                        ui,
                        &join(&ms, "remainingTimeToHide"),
                        &l,
                        &h,
                        left,
                        1.0,
                        |s| Action::SetStrangerTime { seconds: s },
                    );
                }
            }
        });
        ui.add_space(8.0);
    }
    let dc = p(&["DeathclawManager"]);
    if r.at(&dc).is_some() {
        panel(ui, |ui| {
            title(ui, &app.t("o.deathclaw"));
            note(ui, &app.t("o.deathclawHelp"));
            let can = join(&dc, "canDeathclawEmergencyOccurs");
            if r.at(&can).is_some() {
                let o = Opts::text(app.t("o.deathclawCan")).help(app.t("o.deathclawCanHelp"));
                widgets::bool_field(app, ui, &can, o);
            }
            let (l, h) = (app.t("o.deathclawChance"), app.t("o.deathclawChanceHelp"));
            percent_row(app, ui, &join(&dc, "deathclawTotalExtraChance"), &l, &h);
            if let Some(task) = r
                .at(&join(&dc, "deathclawCooldownID"))
                .and_then(Value::as_i64)
                .filter(|t| *t >= 0)
            {
                let (l, h) = (
                    app.t("o.deathclawCooldown"),
                    app.t("o.deathclawCooldownHelp"),
                );
                task_timer(app, ui, task, &l, &h);
            }
        });
        ui.add_space(8.0);
    }
    let em = p(&["vault", "emergencyData"]);
    if r.at(&em).is_some() {
        panel(ui, |ui| incidents(app, ui, &r));
        ui.add_space(8.0);
    }
    let bc = p(&["BottleAndCappyMgrSerializeKey"]);
    if r.at(&bc).is_some() {
        panel(ui, |ui| {
            title(ui, &app.t("o.bottle"));
            note(ui, &app.t("o.bottleHelp"));
            let locked = join(&bc, "SerializeLocked");
            if r.at(&locked).is_some() {
                let o = Opts::text(app.t("o.bottleLocked")).help(app.t("o.bottleLockedHelp"));
                widgets::bool_field(app, ui, &locked, o);
            }
            let (l, h) = (app.t("o.bottleChance"), app.t("o.bottleChanceHelp"));
            percent_row(
                app,
                ui,
                &join(&bc, "SerializeAccumulatedTriggerChance"),
                &l,
                &h,
            );
            if let Some(task) = r
                .at(&join(&bc, "SerializeUnlockTask"))
                .and_then(Value::as_i64)
                .filter(|t| *t >= 0)
            {
                let (l, h) = (app.t("o.bottleUnlockIn"), app.t("o.bottleUnlockInHelp"));
                task_timer(app, ui, task, &l, &h);
            }
        });
        ui.add_space(8.0);
    }
    let rf = p(&["refugeeSpawner"]);
    if r.at(&rf).is_some() {
        panel(ui, |ui| {
            title(ui, &app.t("o.refugees"));
            note(ui, &app.t("o.refugeesHelp"));
            if let Some(pool) = r
                .at(&join(&rf, "currentPool"))
                .and_then(Value::as_text)
                .filter(|s| !s.is_empty())
            {
                let n = pool.trim_start_matches("Pool").to_string();
                info(
                    ui,
                    &app.t("o.refugeesPool"),
                    &app.tf("o.refugeesPoolN", &[("n", n)]),
                );
            }
            let all = join(&rf, "collectedAllRefugees");
            if r.at(&all).is_some() {
                let o = Opts::text(app.t("o.refugeesAll")).help(app.t("o.refugeesAllHelp"));
                widgets::bool_field(app, ui, &all, o);
            }
            for (k, label, help) in [
                ("retrievedDwellers", "o.refugeesGot", "o.refugeesGotHelp"),
                ("failedChances", "o.refugeesFailed", "o.refugeesFailedHelp"),
            ] {
                let path = p(&["refugeeSpawner", "currentPoolData", k]);
                if r.at(&path).is_some() {
                    let o = Opts::text(app.t(label)).help(app.t(help)).range(0.0, 20.0);
                    widgets::num_field(app, ui, &path, o);
                }
            }
            for k in ["globalTimerTask", "newGameTask", "succeed"] {
                if let Some(task) = r
                    .at(&join(&rf, k))
                    .and_then(Value::as_i64)
                    .filter(|t| *t >= 0)
                {
                    let (l, h) = (app.t("o.refugeesIn"), app.t("o.refugeesInHelp"));
                    task_timer(app, ui, task, &l, &h);
                }
            }
        });
        ui.add_space(8.0);
    }
    let rm = p(&["ratingMgr"]);
    if r.at(&rm).is_some() {
        panel(ui, |ui| {
            title(ui, &app.t("o.rating"));
            note(ui, &app.t("o.ratingHelp"));
            let days = vault::arr(&r, &["ratingMgr", "ratings"]);
            if days.is_empty() {
                note(ui, &app.t("o.ratingNone"));
            }
            let body = egui::TextStyle::Body.resolve(ui.style());
            let day_w = (1..=days.len().max(1))
                .map(|n| {
                    widgets::text_width(
                        ui,
                        &app.tf("o.ratingDay", &[("n", n.to_string())]),
                        body.clone(),
                    )
                })
                .fold(0.0_f32, f32::max)
                + 4.0;
            for (k, day) in days.iter().enumerate() {
                ui.horizontal(|ui| {
                    widgets::cell(ui, day_w, egui::Align::Min, |ui| {
                        ui.label(app.tf("o.ratingDay", &[("n", (k + 1).to_string())]));
                    });
                    if let Some(v) = vault::int(day, &["rating"]) {
                        let key = format!("rating:{k}");
                        if let Some(n) = widgets::pick_number(
                            app,
                            ui,
                            &key,
                            v as f64,
                            (0.0, 100.0),
                            Some((0.0, 100.0)),
                            true,
                        ) {
                            if n as i64 != v {
                                app.act(Action::SetDayRating {
                                    index: k,
                                    rating: Some(n as i64),
                                    collected: None,
                                });
                            }
                        }
                        widgets::unit_label(ui, "%");
                    }
                    if let Some(mut got) = vault::flag(day, &["collected"]) {
                        if ui
                            .checkbox(&mut got, app.t("o.ratingCollected"))
                            .on_hover_text(app.t("o.ratingCollectedHelp"))
                            .changed()
                        {
                            app.act(Action::SetDayRating {
                                index: k,
                                rating: None,
                                collected: Some(got),
                            });
                        }
                    }
                });
            }
            ui.add_space(4.0);
            let values: Vec<i64> = days
                .iter()
                .filter_map(|d| vault::int(d, &["rating"]))
                .collect();
            let mut totals: Vec<(String, String, String)> = Vec::new();
            if (1..7).contains(&values.len()) {
                if let Some(avg) = shelter_core::actions::week_average(&values) {
                    totals.push((
                        app.t("o.ratingNow"),
                        format!("{avg}%"),
                        app.t("o.ratingNowHelp"),
                    ));
                }
            }
            for (k, label, help) in [
                ("weekRating", "o.ratingWeek", "o.ratingWeekHelp"),
                ("lastWeekRating", "o.ratingLastWeek", ""),
            ] {
                if let Some(v) = vault::int(&r, &["ratingMgr", k]) {
                    let help = if help.is_empty() {
                        String::new()
                    } else {
                        app.t(help)
                    };
                    totals.push((app.t(label), format!("{v}%"), help));
                }
            }
            let label_w = totals
                .iter()
                .map(|(l, _, _)| widgets::text_width(ui, l, body.clone()))
                .fold(0.0_f32, f32::max)
                + 8.0;
            for (label, value, help) in totals {
                ui.horizontal(|ui| {
                    let r = widgets::cell(ui, label_w, egui::Align::Min, |ui| {
                        ui.label(RichText::new(label).color(style::colors(ui.ctx()).muted))
                    });
                    let v = ui.label(value);
                    if !help.is_empty() {
                        r.on_hover_text(&help);
                        v.on_hover_text(&help);
                    }
                });
            }
        });
        ui.add_space(8.0);
    }
    let cq = p(&["completedQuestDataManager"]);
    if r.at(&cq).is_some() {
        panel(ui, |ui| {
            title(ui, &app.t("o.questTimers"));
            if let Some(task) = r
                .at(&join(&cq, "taskID"))
                .and_then(Value::as_i64)
                .filter(|t| *t >= 0)
            {
                let (l, h) = (app.t("o.newQuestsIn"), app.t("o.newQuestsInHelp"));
                task_timer(app, ui, task, &l, &h);
            }
            let inc = join(&cq, "nukaQuantumIncrement");
            if r.at(&inc).is_some() {
                let o = Opts::text(app.t("o.questQuantum"))
                    .help(app.t("o.questQuantumHelp"))
                    .range(0.0, 10.0);
                widgets::num_field(app, ui, &inc, o);
            }
        });
        ui.add_space(8.0);
    }
    let shop = p(&["ShopWindow"]);
    if let Some(Value::Object(o)) = r.at(&shop) {
        if !o.is_empty() {
            panel(ui, |ui| {
                title(ui, &app.t("o.shop"));
                for k in o.keys() {
                    let path = join(&shop, &k);
                    if let Some(Value::Bool(_)) = r.at(&path) {
                        let opts = Opts::at(app, &path);
                        widgets::bool_field(app, ui, &path, opts);
                    }
                }
            });
            ui.add_space(8.0);
        }
    }
    if app.expert() {
        block(
            app,
            ui,
            "o.eventsRaw",
            &[
                &["MysteriousStranger"],
                &["DeathclawManager"],
                &["BottleAndCappyMgrSerializeKey"],
                &["refugeeSpawner"],
                &["ShopWindow"],
                &["seasonEventData"],
                &["dayToDayRewardMgr"],
                &["PromotionFlags"],
                &["ratingMgr"],
                &["LunchBoxCollectWindow"],
                &["SeasonShopWarningModal"],
                &["PromoCodesWindow"],
                &["JunkGiveAwayWindow"],
                &["swrveEventsManager"],
            ],
        );
    }
}

fn service(app: &mut App, ui: &mut egui::Ui) {
    let mut list: Vec<Vec<String>> = [
        "taskMgr",
        "task",
        "time",
        "timeMgr",
        "cameraPosition",
        "viewManager",
        "localNotificationMgr",
        "happinessManager",
        "constructMgr",
        "survivalW",
        "questSetup",
        "questDataManager",
        "questDwellers",
        "questDwellerSpawner",
        "completedQuestDataManager",
        "objectiveMgr",
        "unlockableMgr",
        "tutorialManager",
        "StatsWindow",
        "DeathclawManager",
        "MysteriousStranger",
    ]
    .iter()
    .map(|k| vec![k.to_string()])
    .collect();
    if let Some(o) = app.open.as_ref().and_then(|o| o.doc.root().as_object()) {
        for k in o.keys() {
            if !SERVICE_KNOWN.contains(&k.as_str()) {
                list.push(vec![k]);
            }
        }
    }
    let refs: Vec<Vec<&str>> = list
        .iter()
        .map(|v| v.iter().map(String::as_str).collect())
        .collect();
    let refs2: Vec<&[&str]> = refs.iter().map(Vec::as_slice).collect();
    note(ui, &app.t("o.serviceHelp"));
    ui.add_space(6.0);
    block(app, ui, "o.service", &refs2);
}

/// Raw editor for whole objects, for expert mode.
fn block(app: &mut App, ui: &mut egui::Ui, title_key: &str, roots: &[&[&str]]) {
    panel(ui, |ui| {
        title(ui, &app.t(title_key));
        for r in roots {
            let path = p(r);
            let Some(v) = app
                .open
                .as_ref()
                .and_then(|o| o.doc.root().at(&path))
                .cloned()
            else {
                continue;
            };
            let o = Opts::at(app, &path);
            match v {
                Value::Object(_) | Value::Array(_) => {
                    let resp = egui::CollapsingHeader::new(o.label.clone())
                        .id_salt(path_to_string(&path))
                        .default_open(false)
                        .show(ui, |ui| {
                            widgets::object_form(app, ui, &path, 0);
                        });
                    if !o.help.is_empty() {
                        resp.header_response.on_hover_text(o.help);
                    }
                }
                Value::Null => {
                    let empty = app.t("c.empty");
                    widgets::row(app, ui, &path, &o, |_, ui| {
                        ui.label(RichText::new(empty).color(style::colors(ui.ctx()).muted));
                    });
                }
                Value::Bool(_) => widgets::bool_field(app, ui, &path, o),
                Value::Number(_) => widgets::num_field(app, ui, &path, o),
                _ => widgets::text_field(app, ui, &path, o),
            }
        }
    });
}
