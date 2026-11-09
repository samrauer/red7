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
pub struct ConservativePalette;

impl Strategy for ConservativePalette {
    fn name(&self) -> &str {
        "conservative-palette"
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

/// Builds its palette hard: prefer a single palette card, then a single
/// canvas card, then both. Palette cards are the highest available; canvas
/// cards (discarded) are the lowest that still win.
pub struct AggressivePalette;

impl Strategy for AggressivePalette {
    fn name(&self) -> &str {
        "aggressive-palette"
    }
    fn choose(&self, _view: &PlayerView, legal: &[Move], _rng: &mut dyn Rng) -> Move {
        legal
            .iter()
            .copied()
            .min_by_key(|mv| match *mv {
                Move::Palette(c) => (0, -i16::from(c), 0),
                Move::Canvas(c) => (1, i16::from(c), 0),
                Move::Both { palette, canvas } => (2, -i16::from(palette), i16::from(canvas)),
            })
            .unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{Color, make_card};
    use crate::game::GameState;
    use rand::SeedableRng;
    use rand::rngs::SmallRng;

    /// Run `strategy` on a hand-built `legal` list. The view is a real but
    /// arbitrary game state; these strategies ignore it.
    fn pick(strategy: &dyn Strategy, legal: &[Move]) -> Move {
        let mut rng = SmallRng::seed_from_u64(0);
        let state = GameState::new(2, &mut rng);
        strategy.choose(&state.view(), legal, &mut rng)
    }

    #[test]
    fn aggressive_prefers_palette_and_plays_highest() {
        let low = make_card(2, Color::Violet);
        let high = make_card(6, Color::Red);
        let legal = [
            Move::Canvas(low),
            Move::Palette(low),
            Move::Both {
                palette: high,
                canvas: low,
            },
            Move::Palette(high),
        ];
        assert_eq!(pick(&AggressivePalette, &legal), Move::Palette(high));
        assert_eq!(pick(&ConservativePalette, &legal), Move::Palette(low));
    }

    #[test]
    fn aggressive_discards_lowest_canvas_card() {
        let low = make_card(3, Color::Blue);
        let high = make_card(3, Color::Orange);
        let legal = [
            Move::Canvas(high),
            Move::Both {
                palette: high,
                canvas: low,
            },
            Move::Canvas(low),
        ];
        assert_eq!(pick(&AggressivePalette, &legal), Move::Canvas(low));
    }

    #[test]
    fn aggressive_both_maximizes_palette_then_minimizes_canvas() {
        let a = make_card(1, Color::Green);
        let b = make_card(4, Color::Yellow);
        let c = make_card(7, Color::Indigo);
        let legal = [
            Move::Both {
                palette: b,
                canvas: a,
            },
            Move::Both {
                palette: c,
                canvas: b,
            },
            Move::Both {
                palette: c,
                canvas: a,
            },
            Move::Both {
                palette: a,
                canvas: c,
            },
        ];
        assert_eq!(
            pick(&AggressivePalette, &legal),
            Move::Both {
                palette: c,
                canvas: a
            }
        );
    }
}
