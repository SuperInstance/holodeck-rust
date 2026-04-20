//! Ten Forward social games
//!
//! Poker game, roundtable debates, storytelling
//! The off-duty social layer of the fleet

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Card {
    pub suit: String,  // hearts, diamonds, clubs, spades
    pub rank: String,  // 2-10, J, Q, K, A
}

impl std::fmt::Display for Card {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let suit_symbol = match self.suit.as_str() {
            "hearts" => "♥",
            "diamonds" => "♦",
            "clubs" => "♣",
            "spades" => "♠",
            _ => "?",
        };
        write!(f, "{}{}", self.rank, suit_symbol)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PokerGame {
    pub pot: u32,
    pub community: Vec<Card>,
    pub hands: HashMap<String, Vec<Card>>,  // player -> hand
    pub deck: Vec<Card>,
    pub phase: String,  // waiting, preflop, flop, turn, river, showdown
    pub dealer: String,
    pub current_bet: u32,
    pub bets: HashMap<String, u32>,
}

impl PokerGame {
    pub fn new() -> Self {
        let mut deck = Vec::new();
        let suits = ["hearts", "diamonds", "clubs", "spades"];
        let ranks = ["2", "3", "4", "5", "6", "7", "8", "9", "10", "J", "Q", "K", "A"];
        for suit in suits {
            for rank in ranks {
                deck.push(Card { suit: suit.to_string(), rank: rank.to_string() });
            }
        }
        // Shuffle using simple method
        let n = deck.len();
        for i in (1..n).rev() {
            // Deterministic-ish shuffle based on position
            let j = (i * 7 + 13) % (i + 1);
            deck.swap(i, j);
        }
        Self {
            pot: 0,
            community: Vec::new(),
            hands: HashMap::new(),
            deck,
            phase: "waiting".to_string(),
            dealer: "House".to_string(),
            current_bet: 0,
            bets: HashMap::new(),
        }
    }

    pub fn join(&mut self, name: &str) -> String {
        if self.phase != "waiting" {
            return format!("Game already in progress ({}). Wait for next hand.", self.phase);
        }
        if self.hands.contains_key(name) {
            return format!("{}, you're already at the table.", name);
        }
        if self.hands.len() >= 6 {
            return "Table's full. 6 players max.".to_string();
        }
        self.hands.insert(name.to_string(), Vec::new());
        self.bets.insert(name.to_string(), 0);
        format!("{} takes a seat. {}/6 players. Type 'deal' when ready.", name, self.hands.len())
    }

    pub fn deal(&mut self) -> String {
        if self.phase != "waiting" {
            return format!("Already dealt. Phase: {}", self.phase);
        }
        if self.hands.len() < 2 {
            return "Need at least 2 players to deal.".to_string();
        }
        // Deal 2 cards to each player
        for hand in self.hands.values_mut() {
            hand.clear();
            if let Some(c1) = self.deck.pop() { hand.push(c1); }
            if let Some(c2) = self.deck.pop() { hand.push(c2); }
        }
        self.phase = "preflop".to_string();
        let players: Vec<String> = self.hands.keys().cloned().collect();
        format!("Cards dealt! Players: {}. Check your hand with 'hand'. Bet, check, or fold.", players.join(", "))
    }

    pub fn show_hand(&self, name: &str) -> String {
        match self.hands.get(name) {
            Some(cards) => {
                if cards.is_empty() {
                    "No cards yet. Wait for deal.".to_string()
                } else {
                    let display: Vec<String> = cards.iter().map(|c| c.to_string()).collect();
                    format!("Your hand: {}", display.join(" "))
                }
            }
            None => "You're not at the table. Type 'join'.".to_string(),
        }
    }

    pub fn flop(&mut self) -> String {
        if self.phase != "preflop" {
            return format!("Not time for flop. Phase: {}", self.phase);
        }
        self.deck.pop(); // burn
        for _ in 0..3 {
            if let Some(c) = self.deck.pop() { self.community.push(c); }
        }
        self.phase = "flop".to_string();
        let display: Vec<String> = self.community.iter().map(|c| c.to_string()).collect();
        format!("Flop: {}  [pot: {}]", display.join(" "), self.pot)
    }

    pub fn turn(&mut self) -> String {
        if self.phase != "flop" {
            return format!("Not time for turn. Phase: {}", self.phase);
        }
        self.deck.pop(); // burn
        if let Some(c) = self.deck.pop() { self.community.push(c); }
        self.phase = "turn".to_string();
        let display: Vec<String> = self.community.iter().map(|c| c.to_string()).collect();
        format!("Turn: {}  [pot: {}]", display.join(" "), self.pot)
    }

    pub fn river(&mut self) -> String {
        if self.phase != "turn" {
            return format!("Not time for river. Phase: {}", self.phase);
        }
        self.deck.pop(); // burn
        if let Some(c) = self.deck.pop() { self.community.push(c); }
        self.phase = "river".to_string();
        let display: Vec<String> = self.community.iter().map(|c| c.to_string()).collect();
        format!("River: {}  [pot: {}]", display.join(" "), self.pot)
    }

    pub fn bet(&mut self, name: &str, amount: u32) -> String {
        if !self.hands.contains_key(name) {
            return "You're not at the table.".to_string();
        }
        *self.bets.get_mut(name).expect("bet: player must exist after contains_key check") += amount;
        self.pot += amount;
        self.current_bet = self.current_bet.max(amount);
        format!("{} bets {}. Pot: {}", name, amount, self.pot)
    }

    pub fn fold(&mut self, name: &str) -> String {
        if self.hands.remove(name).is_some() {
            self.bets.remove(name);
            if self.hands.len() == 1 {
                let winner = self.hands.keys().next().expect("fold: must have 1 remaining player").clone();
                let winnings = self.pot;
                self.pot = 0;
                self.phase = "waiting".to_string();
                self.community.clear();
                return format!("{} folds. {} wins {}! New hand? Type 'deal'.", name, winner, winnings);
            }
            format!("{} folds. {} players remain.", name, self.hands.len())
        } else {
            "You're not in the hand.".to_string()
        }
    }

    pub fn show_table(&self) -> String {
        let display: Vec<String> = self.community.iter().map(|c| c.to_string()).collect();
        let players: Vec<String> = self.hands.keys().cloned().collect();
        format!(
            "Table: {} | Players: {} | Pot: {} | Phase: {}",
            if display.is_empty() { "empty".to_string() } else { display.join(" ") },
            players.join(", "),
            self.pot,
            self.phase
        )
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct StoryCircle {
    pub prompt: String,
    pub contributions: Vec<(String, String)>,  // (agent, text)
    pub max_contributions: usize,
}

#[allow(dead_code)]
impl StoryCircle {
    #[allow(dead_code)]
    pub fn new(prompt: &str, max: usize) -> Self {
        Self {
            prompt: prompt.to_string(),
            contributions: Vec::new(),
            max_contributions: max,
        }
    }

    #[allow(dead_code)]
    pub fn contribute(&mut self, name: &str, text: &str) -> String {
        if self.contributions.len() >= self.max_contributions {
            return "Story circle is full. Read the story with 'story'.".to_string();
        }
        self.contributions.push((name.to_string(), text.to_string()));
        let remaining = self.max_contributions - self.contributions.len();
        if remaining == 0 {
            format!("Story complete! {} contributions. Read with 'story'.", self.contributions.len())
        } else {
            format!("{} added a line. {}/{} lines. Your turn.", name, self.contributions.len(), self.max_contributions)
        }
    }

    #[allow(dead_code)]
    pub fn read(&self) -> String {
        let mut lines = vec![format!("Prompt: {}", self.prompt), String::new()];
        for (name, text) in &self.contributions {
            lines.push(format!("{}: {}", name, text));
        }
        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_poker_join() {
        let mut g = PokerGame::new();
        assert!(g.join("Alice").contains("takes a seat"));
        assert!(g.join("Alice").contains("already"));
    }

    #[test]
    fn test_poker_deal() {
        let mut g = PokerGame::new();
        assert!(g.deal().contains("2 players"));
        g.join("A");
        g.join("B");
        assert!(g.deal().contains("Cards dealt"));
        assert!(g.show_hand("A").contains("Your hand:"));
    }

    #[test]
    fn test_poker_flop() {
        let mut g = PokerGame::new();
        g.join("A");
        g.join("B");
        g.deal();
        assert!(g.flop().contains("Flop:"));
        assert!(g.turn().contains("Turn:"));
        assert!(g.river().contains("River:"));
    }

    #[test]
    fn test_poker_fold_wins() {
        let mut g = PokerGame::new();
        g.join("A");
        g.join("B");
        g.deal();
        g.bet("A", 10);
        assert!(g.fold("B").contains("A wins"));
    }

    #[test]
    fn test_story_circle() {
        let mut s = StoryCircle::new("A ship arrives...", 3);
        s.contribute("A", "at the edge of known space");
        s.contribute("B", "where the stars blink backwards");
        assert!(s.read().contains("blink backwards"));
    }
}
