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

/// A text message from the administrator.
pub struct Command {
    pub text: String,
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

    pub fn send(&self, text: &str) -> Result<(), String> {
        self.call(
            "sendMessage",
            &json!({ "chat_id": self.admin, "text": text, "disable_web_page_preview": true }),
        )
        .map(|_| ())
    }

    /// Waits for new messages; returns those from the administrator's private
    /// chat and the next offset. Everything else is skipped silently.
    pub fn poll(&self, offset: i64) -> Result<(Vec<Command>, i64), String> {
        let v = self.call(
            "getUpdates",
            &json!({ "offset": offset, "timeout": 50, "allowed_updates": ["message"] }),
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
            let Some(m) = u.get("message") else { continue };
            let from = m.pointer("/from/id").and_then(Value::as_i64);
            let chat = m.pointer("/chat/id").and_then(Value::as_i64);
            let private = m.pointer("/chat/type").and_then(Value::as_str) == Some("private");
            if from != Some(self.admin) || chat != Some(self.admin) || !private {
                continue;
            }
            if let Some(text) = m.get("text").and_then(Value::as_str) {
                out.push(Command {
                    text: text.chars().take(200).collect(),
                });
            }
        }
        Ok((out, next))
    }
}
