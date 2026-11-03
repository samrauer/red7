//! Game state and the core turn loop (basic rules, single round).

use rand::Rng;
use rand::seq::SliceRandom;

use crate::card::{Card, CardSet, Color, NUM_CARDS, bit, cards, color};
use crate::rules::score;
use crate::strategy::Strategy;

pub const HAND_SIZE: usize = 7;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Move {
    /// Add a card from hand to your palette.
    Palette(Card),
    /// Play a card from hand to the canvas, changing the rule.
    Canvas(Card),
    /// Add a card to your palette, then change the rule.
    Both { palette: Card, canvas: Card },
}

#[derive(Clone, Debug)]
pub struct GameState {
    hands: Vec<CardSet>,
    palettes: Vec<CardSet>,
    alive: Vec<bool>,
    rule: Color,
    current_player: usize,
}

impl GameState {
    pub fn new(num_players: usize, rng: &mut impl Rng) -> Self {
        assert!((2..=4).contains(&num_players), "Red7 supports 2-4 players");
        let mut deck: Vec<Card> = (0..NUM_CARDS).collect();
        deck.shuffle(rng);
        let mut deck = deck.into_iter();

        let mut hands = vec![0; num_players];
        let mut palettes = vec![0; num_players];
        for p in 0..num_players {
            for _ in 0..HAND_SIZE {
                hands[p] |= bit(deck.next().unwrap());
            }
            palettes[p] = bit(deck.next().unwrap());
        }

        let mut state = GameState {
            hands,
            palettes,
            alive: vec![true; num_players],
            rule: Color::Red,
            current_player: 0,
        };
        // The player to the left of whoever is currently winning goes first.
        let leader = state
            .leader()
            .expect("someone always leads under Red at start");
        state.current_player = state.next_alive(leader);
        state
    }

    pub fn num_players(&self) -> usize {
        self.hands.len()
    }

    pub fn alive_count(&self) -> usize {
        self.alive.iter().filter(|&&a| a).count()
    }

    fn next_alive(&self, from: usize) -> usize {
        let n = self.num_players();
        (1..=n)
            .map(|i| (from + i) % n)
            .find(|&p| self.alive[p])
            .unwrap()
    }

    /// Would `player` be winning with `palette` under `rule`?
    /// Winning means a strictly higher score than every other living player.
    fn would_win(&self, player: usize, palette: CardSet, rule: Color) -> bool {
        let mine = score(palette, rule);
        mine > 0
            && (0..self.num_players())
                .filter(|&j| j != player && self.alive[j])
                .all(|j| score(self.palettes[j], rule) < mine)
    }

    /// The living player currently winning, if any.
    pub fn leader(&self) -> Option<usize> {
        (0..self.num_players())
            .find(|&p| self.alive[p] && self.would_win(p, self.palettes[p], self.rule))
    }

    /// All moves that leave the current player winning.
    /// An empty list means the player must drop out.
    pub fn legal_moves(&self) -> Vec<Move> {
        let p = self.current_player;
        let hand = self.hands[p];
        let pal = self.palettes[p];
        let mut moves = Vec::new();

        for c in cards(hand) {
            if self.would_win(p, pal | bit(c), self.rule) {
                moves.push(Move::Palette(c));
            }
            if self.would_win(p, pal, color(c)) {
                moves.push(Move::Canvas(c));
            }
            for k in cards(hand & !bit(c)) {
                if self.would_win(p, pal | bit(c), color(k)) {
                    moves.push(Move::Both {
                        palette: c,
                        canvas: k,
                    });
                }
            }
        }
        moves
    }

    pub fn apply(&mut self, mv: Move) {
        match mv {
            Move::Palette(c) => self.add_to_palette(c),
            Move::Canvas(c) => self.play_to_canvas(c),
            Move::Both { palette, canvas } => {
                self.add_to_palette(palette);
                self.play_to_canvas(canvas);
            }
        }
    }

    /// Move `card` from the current player's hand to their palette.
    fn add_to_palette(&mut self, card: Card) {
        self.take_from_hand(card);
        self.palettes[self.current_player] |= bit(card);
    }

    /// Play `card` from the current player's hand to the canvas.
    fn play_to_canvas(&mut self, card: Card) {
        self.take_from_hand(card);
        self.rule = color(card);
    }

    fn take_from_hand(&mut self, card: Card) {
        let hand = &mut self.hands[self.current_player];
        debug_assert!(*hand & bit(card) != 0, "card not in hand");
        *hand &= !bit(card);
    }

    pub fn view(&self) -> PlayerView<'_> {
        PlayerView { state: self }
    }
}

/// What a player is allowed to see: their own hand plus all public info.
pub struct PlayerView<'a> {
    state: &'a GameState,
}

impl PlayerView<'_> {
    pub fn me(&self) -> usize {
        self.state.current_player
    }
    pub fn hand(&self) -> CardSet {
        self.state.hands[self.state.current_player]
    }
    pub fn palette(&self, player: usize) -> CardSet {
        self.state.palettes[player]
    }
    pub fn hand_size(&self, player: usize) -> u32 {
        self.state.hands[player].count_ones()
    }
    pub fn is_alive(&self, player: usize) -> bool {
        self.state.alive[player]
    }
    pub fn rule(&self) -> Color {
        self.state.rule
    }
    pub fn num_players(&self) -> usize {
        self.state.num_players()
    }
}

/// Play one full game; returns the index of the winning seat.
pub fn play_game(strategies: &[&dyn Strategy], rng: &mut impl Rng) -> usize {
    let mut state = GameState::new(strategies.len(), rng);
    loop {
        if state.alive_count() == 1 {
            return state.alive.iter().position(|&a| a).unwrap();
        }
        let moves = state.legal_moves();
        if moves.is_empty() {
            state.alive[state.current_player] = false;
        } else {
            let mv = strategies[state.current_player].choose(&state.view(), &moves, rng);
            debug_assert!(moves.contains(&mv), "strategy returned an illegal move");
            state.apply(mv);
        }
        state.current_player = state.next_alive(state.current_player);
    }
}
