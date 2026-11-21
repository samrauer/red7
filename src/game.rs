//! Game state and the core turn loop (basic rules, single round).
//!
//! Setup follows house rules rather than the published ones: 7 cards each,
//! no starting palette card, and seat 0 always goes first.

use rand::Rng;
use rand::seq::SliceRandom;

use crate::card::{Card, CardSet, Color, FULL_DECK, NUM_CARDS, bit, cards, color};
use crate::rules::score;
use crate::strategy::Strategy;

pub const HAND_SIZE: usize = 7;
pub const MIN_PLAYERS: usize = 2;
/// 7 players x 7 cards uses exactly the 49-card deck.
pub const MAX_PLAYERS: usize = 7;
const _: () = assert!(MAX_PLAYERS * HAND_SIZE <= NUM_CARDS as usize);

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
    /// Every card played to the canvas so far (face up, so public).
    canvas: CardSet,
    current_player: usize,
}

impl GameState {
    /// House-rule setup: every player is dealt `HAND_SIZE` cards, palettes
    /// start empty, the canvas starts on Red, and seat 0 goes first.
    pub fn new(num_players: usize, rng: &mut impl Rng) -> Self {
        assert!(
            (MIN_PLAYERS..=MAX_PLAYERS).contains(&num_players),
            "supports {MIN_PLAYERS}-{MAX_PLAYERS} players"
        );
        let mut deck: Vec<Card> = (0..NUM_CARDS).collect();
        deck.shuffle(rng);
        let mut deck = deck.into_iter();

        let mut hands = vec![0; num_players];
        for hand in &mut hands {
            for _ in 0..HAND_SIZE {
                *hand |= bit(deck.next().unwrap());
            }
        }

        // With empty palettes nobody leads, so seat 0 opens by playing any
        // card to their palette (always legal under Red).
        GameState {
            hands,
            palettes: vec![0; num_players],
            alive: vec![true; num_players],
            rule: Color::Red,
            canvas: 0,
            current_player: 0,
        }
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
        self.canvas |= bit(card);
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

    /// Pass the turn to the next living player.
    pub fn end_turn(&mut self) {
        self.current_player = self.next_alive(self.current_player);
    }

    /// Play from the current state until one player remains; returns the
    /// winning seat. `strategies[p]` moves for seat `p`.
    pub fn play_out(&mut self, strategies: &[&dyn Strategy], rng: &mut dyn Rng) -> usize {
        debug_assert_eq!(strategies.len(), self.num_players());
        loop {
            if self.alive_count() == 1 {
                return self.alive.iter().position(|&a| a).unwrap();
            }
            let moves = self.legal_moves();
            if moves.is_empty() {
                self.alive[self.current_player] = false;
            } else {
                let mv = strategies[self.current_player].choose(&self.view(), &moves, rng);
                debug_assert!(moves.contains(&mv), "strategy returned an illegal move");
                self.apply(mv);
            }
            self.end_turn();
        }
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
    /// Every card played to the canvas so far.
    pub fn canvas(&self) -> CardSet {
        self.state.canvas
    }
    /// Cards this player can't see: they are in opponents' hands or were
    /// never dealt.
    pub fn unseen(&self) -> CardSet {
        let palettes = self.state.palettes.iter().fold(0, |a, &p| a | p);
        FULL_DECK & !self.hand() & !palettes & !self.state.canvas
    }
    pub fn num_players(&self) -> usize {
        self.state.num_players()
    }

    /// A full game state consistent with what this player can see: each
    /// opponent's hand is dealt uniformly at random from `unseen()`, keeping
    /// its real size. Everything else is copied as-is (all of it is public).
    pub fn sample_world(&self, rng: &mut dyn Rng) -> GameState {
        let mut pool: Vec<Card> = cards(self.unseen()).collect();
        pool.shuffle(rng);
        let mut pool = pool.into_iter();
        let mut world = self.state.clone();
        // Re-deal every opponent, dead or alive, so no true hand leaks through.
        for p in (0..self.num_players()).filter(|&p| p != self.me()) {
            world.hands[p] = pool
                .by_ref()
                .take(self.hand_size(p) as usize)
                .fold(0, |hand, c| hand | bit(c));
        }
        world
    }
}

/// Play one full game; returns the index of the winning seat.
pub fn play_game(strategies: &[&dyn Strategy], rng: &mut impl Rng) -> usize {
    GameState::new(strategies.len(), rng).play_out(strategies, rng)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::strategy::Random;
    use rand::SeedableRng;
    use rand::rngs::SmallRng;

    #[test]
    fn deal_is_disjoint_with_empty_palettes() {
        let mut rng = SmallRng::seed_from_u64(1);
        for n in MIN_PLAYERS..=MAX_PLAYERS {
            let state = GameState::new(n, &mut rng);
            let mut seen: CardSet = 0;
            for p in 0..n {
                assert_eq!(state.hands[p].count_ones() as usize, HAND_SIZE);
                assert_eq!(seen & state.hands[p], 0, "hands overlap");
                seen |= state.hands[p];
                assert_eq!(state.palettes[p], 0);
            }
            assert_eq!(state.current_player, 0);
            assert_eq!(state.leader(), None);
            if n == MAX_PLAYERS {
                assert_eq!(seen, FULL_DECK, "7 players use the whole deck");
            }
        }
    }

    #[test]
    fn canvas_tracks_played_cards_and_unseen_excludes_public_info() {
        let mut rng = SmallRng::seed_from_u64(3);
        let mut state = GameState::new(3, &mut rng);
        // `apply` doesn't check legality, so any two cards will do.
        let mut hand = cards(state.hands[0]);
        let (pal, canv) = (hand.next().unwrap(), hand.next().unwrap());
        state.apply(Move::Both {
            palette: pal,
            canvas: canv,
        });
        assert_eq!(state.canvas, bit(canv));
        assert_eq!(state.rule, color(canv));

        let view = state.view();
        let unseen = view.unseen();
        assert_eq!(unseen & (view.hand() | view.palette(0) | view.canvas()), 0);
        // Seat 0 can't see the other two hands, nor the 49 - 21 undealt cards.
        let other_hands = state.hands[1] | state.hands[2];
        assert_eq!(unseen & other_hands, other_hands);
        assert_eq!(
            unseen.count_ones(),
            NUM_CARDS as u32 - view.hand().count_ones() - 1 - view.canvas().count_ones()
        );
    }

    #[test]
    fn sample_world_preserves_public_state() {
        let mut rng = SmallRng::seed_from_u64(4);
        let mut state = GameState::new(4, &mut rng);
        for _ in 0..6 {
            let moves = state.legal_moves();
            if moves.is_empty() {
                state.alive[state.current_player] = false;
            } else {
                let mv = Random.choose(&state.view(), &moves, &mut rng);
                state.apply(mv);
            }
            state.end_turn();
        }
        assert!(state.canvas != 0, "want some canvas cards for a real test");

        let me = state.current_player;
        let world = state.view().sample_world(&mut rng);
        assert_eq!(world.hands[me], state.hands[me]);
        assert_eq!(world.palettes, state.palettes);
        assert_eq!(world.alive, state.alive);
        assert_eq!(world.canvas, state.canvas);
        assert_eq!(world.rule, state.rule);
        assert_eq!(world.current_player, me);

        let mut seen = world.canvas;
        for p in 0..4 {
            assert_eq!(world.hands[p].count_ones(), state.hands[p].count_ones());
            for set in [world.hands[p], world.palettes[p]] {
                assert_eq!(seen & set, 0, "cards overlap");
                seen |= set;
            }
        }
    }

    #[test]
    fn sample_world_does_not_depend_on_hidden_hands() {
        let mut rng = SmallRng::seed_from_u64(5);
        let state = GameState::new(3, &mut rng);
        let mut swapped = state.clone();
        swapped.hands.swap(1, 2);

        let a = state.view().sample_world(&mut SmallRng::seed_from_u64(6));
        let b = swapped.view().sample_world(&mut SmallRng::seed_from_u64(6));
        assert_eq!(a.hands, b.hands);
    }

    #[test]
    fn games_finish_at_every_player_count() {
        let mut rng = SmallRng::seed_from_u64(2);
        for n in MIN_PLAYERS..=MAX_PLAYERS {
            let seats: Vec<&dyn Strategy> = vec![&Random; n];
            for _ in 0..200 {
                assert!(play_game(&seats, &mut rng) < n);
            }
        }
    }
}
