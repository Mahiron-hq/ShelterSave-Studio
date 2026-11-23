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

//! Telegram Bot API over outgoing HTTPS only (long polling, no webhook).

use std::time::Duration;

use serde_json::{json, Value};

pub struct Bot {
    agent: ureq::Agent,
    base: String,
    pub admin: i64,
}

/// What arrived from Telegram.
pub enum Update {
    /// A text message from the administrator's private chat.
    Command(String),
    /// A press on an inline button under one of the bot's messages to the administrator.
    Button {
        id: String,
        message: i64,
        data: String,
    },
    /// Someone else wrote to the bot in a private chat.
    Stranger { chat: i64, russian: bool },
}

/// Commands shown in the Telegram menu and suggested while typing "/".
pub const COMMANDS: [(&str, &str); 5] = [
    ("status", "Состояние сайта"),
    ("stats", "Краткая статистика"),
    ("mute", "Приостановить второстепенные оповещения"),
    ("unmute", "Включить все оповещения"),
    ("help", "Справка"),
];

/// Buttons under the input field of the administrator's chat.
pub fn keyboard() -> Value {
    json!({
        "keyboard": [
            [{ "text": "/status" }, { "text": "/stats" }],
            [{ "text": "/mute" }, { "text": "/unmute" }, { "text": "/help" }],
        ],
        "resize_keyboard": true,
        "is_persistent": true,
    })
}

impl Bot {
    pub fn new(token: &str, admin: i64) -> Option<Bot> {
        if token.is_empty() || admin == 0 {
            return None;
        }
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(70)))
            .user_agent("shelter-watch")
            .build()
            .new_agent();
        Some(Bot {
            agent,
            base: format!("https://api.telegram.org/bot{token}"),
            admin,
        })
    }

    fn call(&self, method: &str, body: &Value) -> Result<Value, String> {
        // The token is part of the address, so errors never include the address.
        let mut res = self
            .agent
            .post(&format!("{}/{method}", self.base))
            .send_json(body)
            .map_err(|e| match e {
                ureq::Error::StatusCode(c) => format!("Telegram answered {c}"),
                _ => "Telegram is not reachable".to_string(),
            })?;
        let v: Value = res
            .body_mut()
            .read_json()
            .map_err(|_| "bad answer from Telegram".to_string())?;
        if v.get("ok").and_then(Value::as_bool) == Some(true) {
            Ok(v)
        } else {
            Err("Telegram refused the request".into())
        }
    }

    /// Registers the command menu for the administrator's chat only.
    pub fn setup(&self) -> Result<(), String> {
        let commands: Vec<Value> = COMMANDS
            .iter()
            .map(|(c, d)| json!({ "command": c, "description": d }))
            .collect();
        self.call(
            "setMyCommands",
            &json!({
                "commands": commands,
                "scope": { "type": "chat", "chat_id": self.admin },
            }),
        )?;
        self.call(
            "setChatMenuButton",
            &json!({ "chat_id": self.admin, "menu_button": { "type": "commands" } }),
        )
        .map(|_| ())
    }

    /// Sends a message to the administrator.
    pub fn send(&self, text: &str) -> Result<(), String> {
        self.send_to(self.admin, text, None)
    }

    pub fn send_to(&self, chat: i64, text: &str, markup: Option<Value>) -> Result<(), String> {
        let mut body = json!({ "chat_id": chat, "text": text, "disable_web_page_preview": true });
        if let Some(m) = markup {
            body["reply_markup"] = m;
        }
        self.call("sendMessage", &body).map(|_| ())
    }

    /// Replaces the text of a message after a button press and closes the button's spinner.
    pub fn answer_button(&self, id: &str, message: i64, text: &str) -> Result<(), String> {
        self.call("answerCallbackQuery", &json!({ "callback_query_id": id }))?;
        self.call(
            "editMessageText",
            &json!({ "chat_id": self.admin, "message_id": message, "text": text }),
        )
        .map(|_| ())
    }

    /// Waits for new updates and returns them with the next offset.
    /// Group chats and anything that is not text are skipped.
    pub fn poll(&self, offset: i64) -> Result<(Vec<Update>, i64), String> {
        let v = self.call(
            "getUpdates",
            &json!({
                "offset": offset,
                "timeout": 50,
                "allowed_updates": ["message", "callback_query"],
            }),
        )?;
        let mut next = offset;
        let mut out = Vec::new();
        for u in v
            .get("result")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            if let Some(id) = u.get("update_id").and_then(Value::as_i64) {
                next = next.max(id + 1);
            }
            if let Some(q) = u.get("callback_query") {
                let from = q.pointer("/from/id").and_then(Value::as_i64);
                let message = q.pointer("/message/message_id").and_then(Value::as_i64);
                let id = q.get("id").and_then(Value::as_str);
                let data = q.get("data").and_then(Value::as_str);
                if let (Some(admin), Some(message), Some(id), Some(data)) =
                    (from, message, id, data)
                {
                    if admin == self.admin {
                        out.push(Update::Button {
                            id: id.to_string(),
                            message,
                            data: data.chars().take(64).collect(),
                        });
                    }
                }
                continue;
            }
            let Some(m) = u.get("message") else { continue };
            let from = m.pointer("/from/id").and_then(Value::as_i64);
            let chat = m.pointer("/chat/id").and_then(Value::as_i64);
            let private = m.pointer("/chat/type").and_then(Value::as_str) == Some("private");
            if !private {
                continue;
            }
            if from == Some(self.admin) && chat == Some(self.admin) {
                if let Some(text) = m.get("text").and_then(Value::as_str) {
                    out.push(Update::Command(text.chars().take(200).collect()));
                }
            } else if let Some(chat) = chat {
                let lang = m
                    .pointer("/from/language_code")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                out.push(Update::Stranger {
                    chat,
                    russian: matches!(lang.split('-').next(), Some("ru" | "uk" | "be" | "kk")),
                });
            }
        }
        Ok((out, next))
    }
}
