//! Strategies (bots). Add new ones here and register them in `main.rs`.

use rand::{Rng, RngExt};

use crate::game::{Move, PlayerView};

pub trait Strategy: Sync {
    fn name(&self) -> &str;
    /// Pick one of `legal` (guaranteed non-empty).
    fn choose(&self, view: &PlayerView, legal: &[Move], rng: &mut dyn Rng) -> Move;
}

/// Picks uniformly among legal moves. A baseline every real strategy should beat.
pub struct Random;

impl Strategy for Random {
    fn name(&self) -> &str {
        "random"
    }
    fn choose(&self, _view: &PlayerView, legal: &[Move], rng: &mut dyn Rng) -> Move {
        legal[rng.random_range(0..legal.len())]
    }
}

/// Spends as little as possible: prefer a single palette card, then a single
/// canvas card, then both; within each, give up the lowest card.
pub struct Conservative;

impl Strategy for Conservative {
    fn name(&self) -> &str {
        "conservative"
    }
    fn choose(&self, _view: &PlayerView, legal: &[Move], _rng: &mut dyn Rng) -> Move {
        legal
            .iter()
            .copied()
            .min_by_key(|mv| match *mv {
                Move::Palette(c) => (0, c, 0),
                Move::Canvas(c) => (1, c, 0),
                Move::Both { palette, canvas } => (2, palette.max(canvas), palette.min(canvas)),
            })
            .unwrap()
    }
}
