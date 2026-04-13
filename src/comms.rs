//! Communication — scoped messaging system.
//! say (room), tell (direct), yell (ship-wide), gossip (fleet-wide)
//! All messages persisted to history for agent context recovery.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub sender: String,
    pub channel: Channel,
    pub content: String,
    pub timestamp: i64,
    pub target_room: Option<String>,
    pub target_agent: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Channel {
    Say,      // Room-local
    Tell,     // Direct to agent
    Yell,     // Ship-wide
    Gossip,   // Fleet-wide
    Ooc,      // Out-of-character
    Note,     // Wall note (persistent)
}

impl std::fmt::Display for Channel {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Channel::Say => write!(f, "say"),
            Channel::Tell => write!(f, "tell"),
            Channel::Yell => write!(f, "yell"),
            Channel::Gossip => write!(f, "gossip"),
            Channel::Ooc => write!(f, "ooc"),
            Channel::Note => write!(f, "note"),
        }
    }
}

pub struct CommsSystem {
    pub history: Vec<Message>,
    pub wall_notes: HashMap<String, Vec<WallNote>>, // room_id -> notes
    pub mailboxes: HashMap<String, Vec<MailItem>>,   // agent -> mail
    pub max_history: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WallNote {
    pub id: String,
    pub author: String,
    pub content: String,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MailItem {
    pub id: String,
    pub from: String,
    pub subject: String,
    pub body: String,
    pub timestamp: i64,
    pub read: bool,
}

impl CommsSystem {
    pub fn new() -> Self {
        Self {
            history: Vec::new(),
            wall_notes: HashMap::new(),
            mailboxes: HashMap::new(),
            max_history: 1000,
        }
    }

    pub fn say(&mut self, sender: &str, room_id: &str, content: &str) -> Message {
        let msg = Message {
            id: uuid::Uuid::new_v4().to_string()[..8].to_string(),
            sender: sender.to_string(),
            channel: Channel::Say,
            content: content.to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            target_room: Some(room_id.to_string()),
            target_agent: None,
        };
        self.history.push(msg.clone());
        self.trim_history();
        msg
    }

    pub fn tell(&mut self, sender: &str, target: &str, content: &str) -> Message {
        let msg = Message {
            id: uuid::Uuid::new_v4().to_string()[..8].to_string(),
            sender: sender.to_string(),
            channel: Channel::Tell,
            content: content.to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            target_room: None,
            target_agent: Some(target.to_string()),
        };
        // Deliver to mailbox
        self.mailboxes.entry(target.to_string()).or_default().push(MailItem {
            id: msg.id.clone(),
            from: sender.to_string(),
            subject: "Direct message".to_string(),
            body: content.to_string(),
            timestamp: msg.timestamp,
            read: false,
        });
        self.history.push(msg.clone());
        self.trim_history();
        msg
    }

    pub fn yell(&mut self, sender: &str, content: &str) -> Message {
        let msg = Message {
            id: uuid::Uuid::new_v4().to_string()[..8].to_string(),
            sender: sender.to_string(),
            channel: Channel::Yell,
            content: content.to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            target_room: None,
            target_agent: None,
        };
        self.history.push(msg.clone());
        self.trim_history();
        msg
    }

    pub fn gossip(&mut self, sender: &str, content: &str) -> Message {
        let msg = Message {
            id: uuid::Uuid::new_v4().to_string()[..8].to_string(),
            sender: sender.to_string(),
            channel: Channel::Gossip,
            content: content.to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            target_room: None,
            target_agent: None,
        };
        self.history.push(msg.clone());
        self.trim_history();
        msg
    }

    pub fn write_note(&mut self, room_id: &str, author: &str, content: &str) {
        self.wall_notes.entry(room_id.to_string()).or_default().push(WallNote {
            id: uuid::Uuid::new_v4().to_string()[..8].to_string(),
            author: author.to_string(),
            content: content.to_string(),
            timestamp: chrono::Utc::now().timestamp(),
        });
    }

    pub fn read_notes(&self, room_id: &str) -> Vec<&WallNote> {
        self.wall_notes.get(room_id).map(|n| n.iter().collect()).unwrap_or_default()
    }

    pub fn check_mail(&mut self, agent: &str) -> Vec<MailItem> {
        if let Some(box_) = self.mailboxes.get_mut(agent) {
            let unread: Vec<MailItem> = box_.iter().filter(|m| !m.read).cloned().collect();
            for m in box_.iter_mut() {
                m.read = true;
            }
            unread
        } else {
            Vec::new()
        }
    }

    #[allow(dead_code)]
    pub fn room_messages(&self, room_id: &str, limit: usize) -> Vec<&Message> {
        self.history.iter()
            .filter(|m| m.target_room.as_deref() == Some(room_id) || matches!(m.channel, Channel::Yell | Channel::Gossip))
            .rev()
            .take(limit)
            .collect()
    }

    fn trim_history(&mut self) {
        while self.history.len() > self.max_history {
            self.history.remove(0);
        }
    }
}
