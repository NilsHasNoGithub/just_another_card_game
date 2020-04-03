use crate::sub_vectors_are_same_length;
use rand::prelude::SliceRandom;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;

pub static SUITS: [Suit; 4] = [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades];
// CARD_VALUE_MIN ~ jack
// ...
// CARD_VALUE_MAX ~ 10
static CARD_VALUE_MIN: u8 = 1;
static CARD_VALUE_MAX: u8 = 8;
static POINT_LIMIT: u32 = 15;

#[derive(Serialize, Deserialize, Debug, Copy, Hash, Clone, Eq, PartialEq)]
pub enum Suit {
    Clubs,
    Diamonds,
    Hearts,
    Spades,
}

#[derive(Serialize, Deserialize, Debug, Copy, Hash, Clone, Eq, PartialEq)]
pub struct Card {
    pub suit: Suit,
    pub value: u8,
}

impl Card {
    pub fn new(suit: Suit, value: u8) -> Self {
        Card { suit, value }
    }

    pub fn full_stock() -> Vec<Self> {
        let mut result = Vec::with_capacity(32);
        for suit in &SUITS {
            for value in CARD_VALUE_MIN..=CARD_VALUE_MAX {
                result.push(Self::new(*suit, value));
            }
        }
        result
    }

    pub fn full_stock_shuffled() -> Vec<Self> {
        let mut result = Self::full_stock();
        result.shuffle(&mut rand::thread_rng());
        result
    }
}

impl PartialOrd for Card {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.value.cmp(&other.value))
    }
}

impl Ord for Card {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}

pub fn is_dirty_wash(cards: &[Card]) -> bool {
    let mut seven_occurred = false;
    for card_value in cards.iter().map(|c: &Card| c.value) {
        if card_value == 5 {
            if seven_occurred {
                return false;
            } else {
                seven_occurred = true;
            }
        }
        if card_value > 5 {
            return false;
        }
    }
    true
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    PlayCard(Card),
    RaiseBet,
    AcceptBet,
    Retreat,
    CallDirtyWash,
    ChallengeDirtyWash(usize),
    AllowDirtyWash(usize),
    Wait,
    NotResponded,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerState {
    players: Vec<String>,
    player_indexes: HashMap<String, usize>,
    player_points: Vec<u32>,
    available_actions: Vec<Vec<Action>>,
    round_action_history: Vec<HashMap<String, Action>>,
    round_players: Vec<usize>,
    round_show_open_cards: Vec<usize>,
    round_player_cards: Vec<Vec<Card>>,
    round_stock_cards: Vec<Card>,
    round_last_raising_player: Option<usize>,
    round_current_turn: usize,
    round_bet_current_turn: Option<usize>,
    round_bet: u32,
    round_do_dirty_wash: bool,
    hit_cards_played: Vec<(usize, Card)>,
}

impl ServerState {
    fn is_on_poverty(&self, player: usize) -> bool {
        self.player_points[player] == POINT_LIMIT - 1
    }

    fn players_on_poverty(&self) -> Vec<usize> {
        let mut result = Vec::new();
        for (i, points) in self.player_points.iter().enumerate() {
            if *points == POINT_LIMIT - 1 {
                result.push(i);
            }
        }
        result
    }

    fn is_poverty(&self) -> bool {
        !self.players_on_poverty().is_empty()
    }

    fn draw_card(stock: &mut Vec<Card>, player_cards: &mut Vec<Card>) -> Result<(), &'static str> {
        match stock.pop() {
            None => return Err("04: tried to grab card from empty stock"),
            Some(c) => player_cards.push(c),
        }
        Ok(())
    }

    pub fn new(players: Vec<String>) -> Self {
        let p_len = players.len();
        Self {
            players: players.clone(),
            player_indexes: players
                .into_iter()
                .enumerate()
                .map(|(i, p)| (p, i))
                .collect(),
            player_points: vec![0; p_len],
            available_actions: vec![Vec::new(); p_len],
            round_action_history: Vec::new(),
            round_show_open_cards: Vec::new(),
            round_players: Vec::with_capacity(p_len),
            round_player_cards: Vec::new(),
            round_stock_cards: Card::full_stock_shuffled(),
            round_last_raising_player: None,
            round_current_turn: 0,
            round_bet_current_turn: None,
            round_bet: 1,
            round_do_dirty_wash: true,
            hit_cards_played: Vec::new(),
        }
    }

    fn start_new_round(&mut self) {
        todo!();
    }

    fn round_is_over(&self) -> bool {
        self.round_players.len() <= 1 || self.round_player_cards.iter().all(Vec::is_empty)
    }

    fn round_winner(&self) -> usize {
        assert!(!self.hit_cards_played.is_empty());
        let target_suit = self.hit_cards_played[0].1.suit;
        // Get the player in the game with the highest card of the same suit as the first card
        // Else get the player in the game with the highest card.
        self.hit_cards_played
            .iter()
            .filter(|(p, c)| c.suit == target_suit && self.round_players.contains(p))
            .max_by_key(|(_, c)| *c)
            .unwrap_or_else(|| {
                self.hit_cards_played
                    .iter()
                    .filter(|(p, _)| self.round_players.contains(p))
                    .max_by_key(|(_, c)| *c)
                    .unwrap()
            })
            .0
    }

    fn set_bet_actions(&mut self) {
        if let Some(cur_bet_player) = self.round_bet_current_turn {
            for (i, actions) in self.available_actions.iter_mut().enumerate() {
                if i == cur_bet_player {
                    // Player cannot retreat from his own raise
                    actions.append(&mut vec![Action::AcceptBet, Action::Retreat]);
                } else {
                    actions.push(Action::Wait);
                }
            }
        }
    }

    fn check_current_turn_player(&mut self) {
        let mut dbg_count = 0u32;
        while !self.round_players.contains(&self.round_current_turn) {
            dbg_count += 1;
            if dbg_count > 10e7 as u32 {
                panic!("Stuck in loop")
            }
            self.round_current_turn = (self.round_current_turn + 1) % self.players.len();
        }
    }

    fn do_dirty_wash(&mut self) {
        for (player, actions) in self.available_actions.iter_mut().enumerate() {
            // A player can choose to call dirty wash or wait.
            if self.round_players.contains(&player) {
                actions.push(Action::CallDirtyWash);
            }
            actions.push(Action::Wait);
        }
        self.round_do_dirty_wash = false;
    }

    fn draw_new_cards_for_player(&mut self, player: usize) -> Result<(), &'static str> {
        for _ in 0..4 {
            Self::draw_card(
                &mut self.round_stock_cards,
                &mut self.round_player_cards[player],
            )?;
        }
        *self.player_points.get_mut(player).unwrap() += 1;
        Ok(())
    }
    // TODO:
    // add choosing to leave or stay for raising one by one: make self.round_raise_current_turn
    // Poverty: No dirty wash: set self.do_dirty_wash to false. Make it the turn for the player
    // with poverty. Do artificial raise bet, prevent further bets.
    pub fn next_state(&mut self, actions: HashMap<String, Action>) -> Result<(), &str> {
        // Every player should have an action
        if self.player_indexes.len() != actions.len() {
            return Err("00: Amount of players does not match amount of actions");
        }

        if self.round_players.len() <= 1 {
            self.start_new_round();
            return Ok(());
        }

        // Clear all actions from the previous state
        for a_actions in &mut self.available_actions {
            a_actions.clear();
            a_actions.push(Action::NotResponded);
        }

        let mut ret = false;

        // Query for dirty wash at the start of a round, if there is enough cards in stock
        if self.round_do_dirty_wash && self.round_stock_cards.len() >= 4 {
            self.do_dirty_wash();
        }

        let mut cur_dw_player = None;
        let mut allow_dirty_wash_amount: usize = 0;

        for (player, action) in actions.iter() {
            if *action == Action::CallDirtyWash {
                let p_idx = self.player_indexes[player];
                // Ask everyone to challenge
                for a_actions in &mut self.available_actions {
                    a_actions.push(Action::ChallengeDirtyWash(p_idx));
                    a_actions.push(Action::Wait);
                }

                ret = true;
                break;
            } else if let Action::ChallengeDirtyWash(challenged_player) = action {
                // If someone has challenged dirty wash, If the challenged player has dirty wash, he plays with
                // open cards and gets a point. Otherwise, the challenger gets a point. do dirty wash again, so other people can do dirty wash as well

                if is_dirty_wash(&self.round_player_cards[*challenged_player]) {
                    self.draw_new_cards_for_player(*challenged_player)?;
                } else {
                    self.round_show_open_cards.push(*challenged_player);
                    *self.player_points.get_mut(*challenged_player).unwrap() += 1;
                }

                ret = true;
                self.do_dirty_wash();
                break;
            } else if let Action::AllowDirtyWash(p) = action {
                allow_dirty_wash_amount += 1;
                match cur_dw_player {
                    None => cur_dw_player = Some(p),
                    Some(p2) => {
                        if p != p2 {
                            return Err("05: ");
                        }
                    }
                }
            }

            if allow_dirty_wash_amount >= self.round_players.len() {
                self.draw_new_cards_for_player(*cur_dw_player.unwrap())?;
            }
        }

        // If someone has challenged dirty wash, If the challenged player has dirty wash, he plays with
        // open cards and gets a point. Otherwise, the challenger gets a point. do dirty wash again, so other people can do dirty wash as well

        // If no one has challenged the dirty wash, the player gets new cards

        // Set history and break if someone has dirty washed
        if ret {
            self.round_action_history.push(actions);
            return Ok(());
        }

        // If all cards are played, determine the player who played the highest card. Increment points of all other players. Start new round.
        if self.round_player_cards.iter().all(Vec::is_empty) {
            if self.hit_cards_played.is_empty() {
                return Err("02: No cards have been played, yet no one has cards.");
            }
            let winner: usize = self.round_winner();
            for player in &self.round_players {
                if winner != *player {
                    self.player_points[*player] += self.round_bet;
                }
            }
            self.start_new_round();
            return Ok(());
        }

        // If a hit is complete, that is if and only if cards have been played, and all players have an equal amount of cards
        if !self.hit_cards_played.is_empty()
            && sub_vectors_are_same_length(&self.round_player_cards)
        {
            // The turn goes to the player who won the hit
            self.round_current_turn = self.round_winner();
            // Clear hit_cards_played
            self.hit_cards_played.clear();
        }

        // If a player is raising, set all available actions for other players to AcceptBet or Retreat, and return
        for (player, action) in actions.iter() {
            if *action == Action::RaiseBet {
                let player_idx = self.player_indexes[player];

                // Set current raising player
                self.round_last_raising_player = Some(self.player_indexes[player]);

                // Set to break loop
                ret = true;

                // Set bet turn
                self.round_bet_current_turn = Some(
                    self.round_players[(self
                        .round_players
                        .iter()
                        .position(|p| *p == player_idx)
                        .unwrap()
                        + 1)
                        % self.round_players.len()],
                );

                // Set all available actions for other players
                self.set_bet_actions();

                break;
            }
        }

        // Set history and break if someone has raised
        if ret {
            self.round_action_history.push(actions);
            return Ok(());
        }

        if let Some(p) = self.round_bet_current_turn {
            if p == self.round_last_raising_player.unwrap() {
                self.round_bet_current_turn = None;
            } else {
                ret = true;
                let action = actions[&self.players[p]];
                // Only if the player is still playing
                let mut increment_bet_turn = false;
                match action {
                    Action::Retreat => {
                        // Add points to player
                        self.player_points[p] += self.round_bet;
                        // Remove p
                        self.round_players.retain(|p2| *p2 != p);
                        // Increment bet turn
                        increment_bet_turn = true;
                    }
                    Action::AcceptBet => increment_bet_turn = true,
                    Action::NotResponded => {}
                    _ => {
                        return Err("01: got an action which was not expected, while raise was done in the previous state.");
                    }
                }

                if increment_bet_turn {
                    self.round_bet_current_turn = Some(
                        self.round_players[(self
                            .round_players
                            .iter()
                            .position(|p2| p == *p2)
                            .unwrap()
                            + 1)
                            % self.round_players.len()],
                    );
                    self.set_bet_actions();
                }
            }
        }

        if ret {
            self.round_action_history.push(actions);
            return Ok(());
        }

        // If a card has been played: Add the player and the card to hit_cards_played. The turn goes to the next player.
        for (player, action) in &actions {
            if let Action::PlayCard(card) = action {
                let mut err = false;
                let idx = self.player_indexes[player];
                // Add to hit_cards played
                self.hit_cards_played.push((idx, *card));
                // Turn goes to next player
                self.round_current_turn = self.round_players[(self
                    .round_players
                    .iter()
                    .position(|p| *p == idx)
                    .unwrap_or_else(|| {
                        err = true;
                        0
                    })
                    + 1)
                    % self.round_players.len()];
                // Return an error if an error occurred
                if err {
                    return Err("03: Presumably a player who was not in the round played a card");
                }
                // Only one player is able to play a card if this program works correctly
                break;
            }
        }

        self.check_current_turn_player();

        // Now all actions for the remaining players should be determined
        for player in 0..self.player_indexes.len() {
            let aa_player: &mut Vec<Action> = self.available_actions.get_mut(player).unwrap();

            // If the player is in the round
            if self.round_players.contains(&player) {
                let player_cards: &[Card] = &self.round_player_cards[player];

                // A player whose turn it is, must play a card of the same suit as the first played
                // card of the hit if he possesses that suit, otherwise, he can play any card.
                if player == self.round_current_turn {
                    // If cards have been played already, add PlayCard for any card with the same suit as the first played card
                    let mut card_actions = Vec::new();
                    if !self.hit_cards_played.is_empty() {
                        let target_suit: Suit = self.hit_cards_played[0].1.suit;
                        for card in player_cards {
                            if card.suit == target_suit {
                                card_actions.push(Action::PlayCard(*card));
                            }
                        }
                    }
                    // If there weren't any cards played, or the player did not have any card of the same suit, the player can choose one from all it's cards
                    if self.hit_cards_played.is_empty() || card_actions.is_empty() {
                        for card in player_cards {
                            card_actions.push(Action::PlayCard(*card));
                        }
                    }

                    // Add all card actions to the player actions
                    aa_player.append(&mut card_actions);
                } else {
                    // A player whose turn it not is can wait
                    aa_player.push(Action::Wait)
                }

                // Any player in the round can raise the bet at any time if the player is not the one who raised the bet previously
                if match self.round_last_raising_player {
                    None => true,
                    Some(p) => player != p,
                } {
                    aa_player.push(Action::RaiseBet);
                }
            } else {
                // Players outside the round can only wait
                aa_player.push(Action::Wait)
            }
        }

        self.round_action_history.push(actions);
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerState {
    players: Vec<String>,
    player_points: Vec<u32>,
    available_actions: Vec<Action>,
    round_action_history: Vec<Action>,
    round_cards: Vec<Card>,
    round_players: Vec<usize>,
    round_stock_card_amount: u32,
    round_bet: u32,
}

impl PlayerState {}
