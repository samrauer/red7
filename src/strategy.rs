//! Strategies (bots). Add new ones here and register them in `main.rs`.

use std::cmp::Reverse;

use rand::{Rng, RngExt};

use crate::game::{MAX_PLAYERS, Move, PlayerView};

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

/// Flat Monte Carlo. Deals opponents random hands from the unseen cards
/// `samples` times, plays out each legal move to the end of the round with
/// every seat following `playout_strategy`, and picks the move that won most
/// often (ties: earliest in `legal`).
///
/// TODO: Sample hands consistent with opponents' past moves.
pub struct MonteCarlo {
    samples: u32,
    /// Strategy every seat follows when playing a sampled world to the end.
    playout_strategy: &'static dyn Strategy,
    name: String,
}

impl MonteCarlo {
    pub fn new(samples: u32, playout_strategy: &'static dyn Strategy) -> Self {
        let name = format!("monte-carlo-{}", playout_strategy.name());
        MonteCarlo {
            samples,
            playout_strategy,
            name,
        }
    }
}

impl Strategy for MonteCarlo {
    fn name(&self) -> &str {
        &self.name
    }
    fn choose(&self, view: &PlayerView, legal: &[Move], rng: &mut dyn Rng) -> Move {
        if legal.len() == 1 {
            return legal[0];
        }
        let seats = [self.playout_strategy; MAX_PLAYERS];
        let seats = &seats[..view.num_players()];
        let mut wins = vec![0u32; legal.len()];
        for _ in 0..self.samples {
            // Every move is scored on the same worlds, so differences between
            // moves aren't drowned out by differences between deals.
            let world = view.sample_world(rng);
            for (&mv, w) in legal.iter().zip(&mut wins) {
                let mut game = world.clone();
                game.apply(mv);
                game.end_turn();
                if game.play_out(seats, rng) == view.me() {
                    *w += 1;
                }
            }
        }
        let best = (0..legal.len())
            .max_by_key(|&i| (wins[i], Reverse(i)))
            .unwrap();
        legal[best]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{Color, make_card};
    use crate::game::{GameState, play_game};
    use rand::SeedableRng;
    use rand::rngs::SmallRng;

    /// Run `strategy` on a hand-built `legal` list. The view is a real but
    /// arbitrary game state; these strategies ignore it.
    fn pick(strategy: &dyn Strategy, legal: &[Move]) -> Move {
        let mut rng = SmallRng::seed_from_u64(0);
        let state = GameState::new(2, &mut rng);
        strategy.choose(&state.view(), legal, &mut rng)
    }

    /// Play `mc` heads-up against `Random` and assert it wins at least 75%.
    fn assert_monte_carlo_beats_random_heads_up(mc: &MonteCarlo) {
        let mut rng = SmallRng::seed_from_u64(7);
        let games = 200;
        let mut mc_wins = 0;
        for g in 0..games {
            // Alternate seats so moving first favors neither side.
            let mc_seat = g % 2;
            let mut seats: [&dyn Strategy; 2] = [&Random, &Random];
            seats[mc_seat] = mc;
            if play_game(&seats, &mut rng) == mc_seat {
                mc_wins += 1;
            }
        }
        assert!(
            mc_wins >= 150,
            "{} won only {mc_wins}/{games} vs random",
            mc.name()
        );
    }

    #[test]
    fn monte_carlo_with_random_playouts_beats_random_heads_up() {
        assert_monte_carlo_beats_random_heads_up(&MonteCarlo::new(8, &Random));
    }

    #[test]
    fn monte_carlo_with_conservative_playouts_beats_random_heads_up() {
        assert_monte_carlo_beats_random_heads_up(&MonteCarlo::new(8, &ConservativePalette));
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
