use crate::data_structures::BullyingAction::{Wait, DrawCard};
use crate::data_structures::GameState::Bullying;
use core::mem;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::RandomState;
use std::collections::HashMap;
use std::ops::Range;

#[non_exhaustive]
#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone)]
pub enum GameStatePlayer {
    Menu(MenuState),
    Bullying(GameStateBullying),
}

#[non_exhaustive]
#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone)]
pub enum GameState {
    Menu(MenuState),
    Bullying(GameStateBullying),
}

#[derive(Serialize, Deserialize, Debug, Hash, Eq, PartialEq, Clone, Constructor)]
pub struct MenuState {
    players: Vec<Player>,
}

#[derive(Serialize, Deserialize, Debug, Hash, PartialEq, Eq, Clone)]
pub struct Player {
    id: String,
}

pub static SUITS: [Suit; 4] = [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades];
pub static CARD_VALUE_MIN: u8 = 1;
pub static CARD_VALUE_MAX: u8 = 14;

#[derive(Serialize, Deserialize, Debug, Copy, Hash, Clone, Eq, PartialEq)]
pub enum Suit {
    Clubs,
    Diamonds,
    Hearts,
    Spades,
}

#[derive(Serialize, Deserialize, Debug, Copy, Hash, Clone, Eq, PartialEq)]
pub enum Card {
    Joker,
    Other { suit: Suit, value: u8 },
}

impl Card {
    pub fn new(suit: Suit, value: u8) -> Card {
        Card::Other { suit, value }
    }

    pub fn new_joker() -> Card {
        Card::Joker
    }

    pub fn full_stock(n_jokers: u32) -> Vec<Card> {
        let mut stock = Vec::with_capacity((52 + n_jokers) as usize);
        {
            for suit in SUITS.iter() {
                for value in CARD_VALUE_MIN..=CARD_VALUE_MAX {
                    stock.push(Card::Other {
                        suit: suit.clone(),
                        value,
                    })
                }
            }
        }
        stock
    }

    pub fn n_full_stocks(n: u32, n_jokers_per_stock: u32) -> Vec<Card> {
        let mut stock = Vec::with_capacity(((52 + n_jokers_per_stock) * n) as usize);
        for _ in 0..n {
            stock.append(&mut Self::full_stock(n_jokers_per_stock));
        }
        stock
    }

    pub fn bullying_is_bully_card(&self) -> bool {
        match self {
            Card::Joker => true,
            Card::Other { value, .. } => match value {
                1 | 2 | 7 | 8 | 10 | 11 => true,
                _ => false,
            },
        }
    }

    pub fn bullying_get_draw_amount(&self) -> u32 {
        match self {
            Card::Joker => 5,
            Card::Other { value, .. } => match value {
                2 => 2,
                _ => 0,
            },
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Hash, Eq, PartialEq, Clone)]
pub enum BullyingAction {
    Wait,
    DrawCard,
    DrawBullyCards(u32),
    PlayCard(Card),
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone)]
pub struct GameStateBullyingPlayer {
    players: Vec<Player>,
    games_won_of_players: HashMap<Player, u32>,
    cards: Vec<Card>,
    card_amount_of_players: HashMap<Player, u32>,
    last_played_card: Option<Card>,
    available_moves: Vec<BullyingAction>,
    game_over: bool,
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone)]
pub struct GameStateBullying {
    players: Vec<Player>,
    games_won_of_players: HashMap<Player, u32>,
    cards_in_stock: Vec<Card>,
    cards_played: Vec<Card>,
    cards_of_players: HashMap<Player, Vec<Card>>,
    available_moves_of_players: HashMap<Player, Vec<BullyingAction>>,
    current_turn: Player,
    previous_non_wait_action: (Player, BullyingAction),
    draw_stack: u32,
    going_clock_wise: bool,
}

impl GameStateBullying {
    pub fn new_game_default(players: Vec<Player>) -> GameStateBullying {
        let l = players.len() as f32;
        Self::new_game(players, (l / 5.0).ceil() as u32, 2, 7)
    }

    pub fn playable_cards(
        draw_stack: u32,
        last_played_card: Card,
        cards_in_hand: &[Card],
    ) -> Vec<Card> {
        cards_in_hand
            .iter()
            .filter(|c| match c {
                Card::Joker => true,
                Card::Other { suit, value } => {
                    if draw_stack > 0 {
                        return value == &2;
                    }
                    if value == &11 {
                        return true;
                    }
                    match last_played_card {
                        Card::Joker => true,
                        Card::Other {
                            suit: suit2,
                            value: value2,
                        } => suit == &suit2 || value == &value2,
                    }
                }
            })
            .copied()
            .collect()
    }

    pub fn available_actions(&self, player: &Player) -> Vec<BullyingAction> {
        if &self.current_turn == player {
            let pc = Self::playable_cards(
                self.draw_stack,
                *self.cards_played.last().unwrap(),
                &self.cards_of_players[player],
            );
            let mut pc: Vec<_> = pc.into_iter().map(BullyingAction::PlayCard).collect();

            if &self.previous_non_wait_action.0 != player
                || match &self.previous_non_wait_action.1 {
                    BullyingAction::PlayCard(_) | BullyingAction::DrawBullyCards(_)=> true,

                    _ => false,
                }
            {

                if self.draw_stack > 0 {
                    pc.push(BullyingAction::DrawBullyCards(self.draw_stack));
                } else if pc.is_empty() {
                    return vec![BullyingAction::DrawCard];
                }
                return pc;
            } else {
                pc.push(BullyingAction::Wait);
                return pc;
            }
        }
        vec![BullyingAction::Wait]
    }

    pub fn new_game(
        players: Vec<Player>,
        n_stocks: u32,
        n_jokers_per_stock: u32,
        n_cards_per_player: u32,
    ) -> GameStateBullying {
        assert!(n_stocks > 0);
        assert!(n_cards_per_player > 0);
        assert!(players.len() >= 2);

        let mut games_won_of_players = HashMap::with_capacity(players.len());
        for player in &players {
            games_won_of_players.insert(player.clone(), 0);
        }

        let mut cards_in_stock = Card::n_full_stocks(n_stocks, n_jokers_per_stock);
        cards_in_stock.shuffle(&mut rand::thread_rng());

        let mut cards_of_players = HashMap::with_capacity(players.len());
        for player in &players {
            let mut cards = Vec::with_capacity(n_cards_per_player as usize);
            for _ in 0..n_cards_per_player {
                cards.push(cards_in_stock.pop().unwrap());
            }
            cards_of_players.insert(player.clone(), cards);
        }

        let current_turn = players[0].clone();
        let cards_played = vec![cards_in_stock.pop().unwrap()];

        let mut available_moves_of_players: HashMap<Player, Vec<BullyingAction>> =
            HashMap::with_capacity(players.len());
        for player in &players {
            if player == &current_turn {
                let mut am: Vec<_> = Self::playable_cards(
                    cards_played[0].bullying_get_draw_amount(),
                    cards_played[0],
                    &cards_of_players[player],
                )
                .iter()
                .map(|c| BullyingAction::PlayCard(*c))
                .collect();
                if am.is_empty() {
                    am.push(BullyingAction::DrawCard)
                }
                available_moves_of_players.insert(player.clone(), am);
            } else {
                available_moves_of_players.insert(player.clone(), vec![BullyingAction::Wait]);
            }
        }

        GameStateBullying {
            players,
            games_won_of_players,
            cards_in_stock,
            cards_played,
            cards_of_players,
            available_moves_of_players,
            current_turn: current_turn.clone(),
            previous_non_wait_action: (current_turn, BullyingAction::Wait),
            draw_stack: 0,
            going_clock_wise: true,
        }
    }

    pub fn get_player_states(&self) -> HashMap<Player, GameStateBullyingPlayer> {
        let mut result = HashMap::with_capacity(self.players.len());
        for player in &self.players {
            result.insert(
                player.clone(),
                GameStateBullyingPlayer {
                    players: self.players.clone(),
                    games_won_of_players: self.games_won_of_players.clone(),
                    cards: self.cards_of_players[player].clone(),
                    card_amount_of_players: self
                        .cards_of_players
                        .iter()
                        .map(|(k, v)| (k.clone(), v.len() as u32))
                        .collect(),
                    last_played_card: self.cards_played.last().cloned(),
                    available_moves: self.available_moves_of_players[player].clone(),
                    game_over: self.cards_of_players.iter().any(|(_, v)| v.is_empty()),
                },
            );
        }
        result
    }

    pub fn draw_card(&mut self) -> Card {
        match self.cards_in_stock.pop() {
            Some(c) => c,
            None => {
                while self.cards_played.len() > 1 {
                    self.cards_in_stock.push(self.cards_played.remove(0));
                }
                self.cards_in_stock.shuffle(&mut rand::thread_rng());
                self.cards_in_stock.pop().unwrap()
            }
        }
    }

    pub fn next_state(&mut self, actions: &HashMap<Player, BullyingAction>) -> Result<(), &str> {
        let players = &self.players as *const Vec<Player>;
        for player in unsafe { &*players } {
            let action = &actions[player];
            if !self.available_moves_of_players[player].contains(action) {
                return Err("Unexpected move in action set");
            }
            match action {
                BullyingAction::Wait => {}
                BullyingAction::DrawBullyCards(n) => {
                    for _ in 0..*n {
                        let c = self.draw_card();
                        self.cards_of_players.get_mut(player).unwrap().push(c);
                    }
                    self.previous_non_wait_action = (player.clone(), action.clone());
                }
                BullyingAction::PlayCard(c) => {
                    match c {
                        Card::Joker => {}
                        Card::Other { .. } => {}
                    }
                    self.previous_non_wait_action = (player.clone(), action.clone());
                }
                BullyingAction::DrawCard => {
                    let c = self.draw_card();
                    self.cards_of_players.get_mut(player).unwrap().push(c);
                    self.previous_non_wait_action = (player.clone(), action.clone());
                }
            }
        }
        Ok(())
    }
}

pub(crate) mod tests {
    use crate::data_structures::{GameStateBullying, Player};

    #[test]
    fn test_bullying() {
        println!("test bullying ----------------------------");
        let players = vec!["a", "b", "c"];
        let players: Vec<_> = players
            .iter()
            .map(|s| Player {
                id: (*s).to_string(),
            })
            .collect();
        let game = GameStateBullying::new_game_default(players);
        let player_games = game.get_player_states();
        println!(
            "Game state server:\n{}\n------------------\n",
            serde_yaml::to_string(&game).unwrap()
        );
        println!(
            "Game states players:\n{}\n------------------\n",
            serde_yaml::to_string(&player_games).unwrap()
        );
        let game_encoded = bincode::serialize(&game).unwrap();
        let player_games_encoded = bincode::serialize(&player_games).unwrap();
        assert_eq!(game, bincode::deserialize(&game_encoded).unwrap());
        assert_eq!(
            player_games,
            bincode::deserialize(&player_games_encoded).unwrap()
        );
    }
}
