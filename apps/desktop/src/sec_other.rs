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
use shelter_core::catalog::catalog;
use shelter_core::i18n::Lang;
use shelter_core::json::{path_to_string, Seg, Value};
use shelter_core::vault::{self, p};

use crate::app::{panel, section_title, App, OtherTab};
use crate::icons;
use crate::style;
use crate::widgets::{self, Opts};

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    section_title(ui, icons::LAYERS, &app.t("nav.other"), "");
    ui.horizontal_wrapped(|ui| {
        for (tab, key) in [
            (OtherTab::Quests, "o.quests"),
            (OtherTab::Wasteland, "o.wasteland"),
            (OtherTab::Objectives, "o.objectives"),
            (OtherTab::Stats, "o.stats"),
            (OtherTab::Tutorial, "o.tutorial"),
            (OtherTab::Events, "o.events"),
            (OtherTab::Service, "o.service"),
            (OtherTab::Tree, "o.tree"),
        ] {
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
        .auto_shrink(false)
        .show(ui, |ui| match app.other_tab {
            OtherTab::Quests => quests(app, ui),
            OtherTab::Wasteland => block(app, ui, "o.wasteland", &[&["vault", "wasteland"]]),
            OtherTab::Objectives => objectives(app, ui),
            OtherTab::Stats => block(app, ui, "o.stats", &[&["StatsWindow"]]),
            OtherTab::Tutorial => {
                if ui
                    .button(format!("{}  {}", icons::CHECK, app.t("o.completeTutorial")))
                    .clicked()
                {
                    app.act(Action::CompleteTutorial);
                }
                block(app, ui, "o.tutorial", &[&["tutorialManager"]]);
            }
            OtherTab::Events => block(
                app,
                ui,
                "o.events",
                &[
                    &["DeathclawManager"],
                    &["MysteriousStranger"],
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
            ),
            OtherTab::Service => {
                let known = [
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
                ]
                .iter()
                .map(|k| vec![k.to_string()])
                .collect();
                if let Some(o) = app.open.as_ref().and_then(|o| o.doc.root().as_object()) {
                    for k in o.keys() {
                        if !known.contains(&k.as_str()) {
                            list.push(vec![k]);
                        }
                    }
                }
                let refs: Vec<Vec<&str>> = list
                    .iter()
                    .map(|v| v.iter().map(String::as_str).collect())
                    .collect();
                let refs2: Vec<&[&str]> = refs.iter().map(Vec::as_slice).collect();
                block(app, ui, "o.service", &refs2);
            }
            OtherTab::Tree => {}
        });
}

fn block(app: &mut App, ui: &mut egui::Ui, title: &str, roots: &[&[&str]]) {
    panel(ui, |ui| {
        ui.label(RichText::new(app.t(title)).font(style::bold(16.0)));
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
            let label = o.label.clone();
            match v {
                Value::Object(_) | Value::Array(_) => {
                    egui::CollapsingHeader::new(label)
                        .id_salt(path_to_string(&path))
                        .default_open(roots.len() == 1)
                        .show(ui, |ui| {
                            widgets::object_form(app, ui, &path, 0);
                        });
                }
                Value::Null => {
                    ui.label(format!("{label}: null"));
                }
                _ => match v {
                    Value::Bool(_) => widgets::bool_field(app, ui, &path, o),
                    Value::Number(_) => widgets::num_field(app, ui, &path, o),
                    _ => widgets::text_field(app, ui, &path, o),
                },
            }
        }
    });
}

fn name_list(
    app: &App,
    ui: &mut egui::Ui,
    ids: &[String],
    lookup: impl Fn(&str) -> Option<String>,
) {
    let c = style::colors(ui.ctx());
    egui::ScrollArea::vertical()
        .id_salt(ui.next_auto_id())
        .max_height(260.0)
        .show(ui, |ui| {
            for id in ids {
                ui.horizontal(|ui| {
                    ui.label(lookup(id).unwrap_or_else(|| id.clone()));
                    ui.label(RichText::new(id).font(style::mono(11.0)).color(c.muted));
                });
            }
        });
    let _ = app;
}

fn quests(app: &mut App, ui: &mut egui::Ui) {
    let Some(open) = &app.open else { return };
    let root = open.doc.root().clone();
    let ru = app.lang == Lang::Ru;
    let quest_name = |id: &str| {
        catalog()
            .quests
            .iter()
            .find(|q| q.id == id)
            .map(|q| q.name.get(ru).to_string())
    };
    let done: Vec<String> = vault::arr(&root, &["completedQuestDataManager", "completedQuests"])
        .iter()
        .filter_map(Value::as_text)
        .collect();
    panel(ui, |ui| {
        ui.label(
            RichText::new(app.tf("o.completedQuests", &[("n", done.len().to_string())]))
                .font(style::bold(16.0)),
        );
        name_list(app, ui, &done, quest_name);
    });
    ui.add_space(8.0);
    for (title, path) in [
        (
            "o.daily",
            &["completedQuestDataManager", "dailyQuestPicker"][..],
        ),
        (
            "o.weekly",
            &["completedQuestDataManager", "weeklyQuestPicker"][..],
        ),
    ] {
        panel(ui, |ui| {
            ui.label(RichText::new(app.t(title)).font(style::bold(16.0)));
            widgets::object_form(app, ui, &p(path), 0);
        });
        ui.add_space(8.0);
    }
    block(
        app,
        ui,
        "o.currentQuest",
        &[&["questDataManager"], &["questSetup"], &["task"], &["time"]],
    );
    ui.add_space(8.0);
    block(
        app,
        ui,
        "o.questDwellers",
        &[&["questDwellers"], &["questDwellerSpawner"]],
    );
    ui.add_space(8.0);
    block(app, ui, "o.quests", &[&["completedQuestDataManager"]]);
}

fn objectives(app: &mut App, ui: &mut egui::Ui) {
    let Some(open) = &app.open else { return };
    let root = open.doc.root().clone();
    let ru = app.lang == Lang::Ru;
    let done: Vec<String> = vault::arr(&root, &["vault", "Achievements", "completed"])
        .iter()
        .filter_map(Value::as_text)
        .collect();
    panel(ui, |ui| {
        ui.label(
            RichText::new(format!("{} {}", icons::BADGE, app.t("o.objectives")))
                .font(style::bold(16.0)),
        );
        name_list(app, ui, &done, |id| {
            catalog()
                .achievements
                .iter()
                .find(|a| a.id == id)
                .map(|a| a.name.get(ru).to_string())
        });
        let slots = vault::arr(&root, &["objectiveMgr", "slotArray"]);
        for s in slots {
            if let Some(id) = s
                .at(&p(&["objective", "objectiveID"]))
                .and_then(Value::as_text)
            {
                let name = catalog()
                    .objectives
                    .iter()
                    .find(|o| o.id == id)
                    .map_or(id.clone(), |o| o.name.get(ru).replace("{0}", "N"));
                ui.label(format!("{}  {name}", icons::FLAG));
            }
        }
    });
    ui.add_space(8.0);
    block(
        app,
        ui,
        "o.objectives",
        &[
            &["objectiveMgr"],
            &["vault", "Achievements"],
            &["unlockableMgr"],
        ],
    );
    let _ = Seg::Index(0);
}
