//! Strategies (bots). Add new ones here and register them in `main.rs`.

use std::cmp::Reverse;

use rand::{Rng, RngExt};

use crate::card::{CardSet, Color};
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
                Move::Palette(c) => (0, c.id(), 0),
                Move::Canvas(c) => (1, c.id(), 0),
                Move::Both { palette, canvas } => {
                    (2, palette.max(canvas).id(), palette.min(canvas).id())
                }
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
                Move::Palette(c) => (0, -i16::from(c.id()), 0),
                Move::Canvas(c) => (1, i16::from(c.id()), 0),
                Move::Both { palette, canvas } => {
                    (2, -i16::from(palette.id()), i16::from(canvas.id()))
                }
            })
            .unwrap()
    }
}

/// What to optimize first when ranking moves in `RuleCoverage`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoveragePriority {
    /// Most rules led, then fewest cards spent.
    CoverageFirst,
    /// Fewest cards spent, then most rules led.
    CardsFirst,
}

/// Maximizes rule coverage: the number of the 7 canvas rules under which its
/// palette (after the move) would be winning. A palette that leads under many
/// rules is hard to overtake by changing the rule. Remaining ties go to the
/// move that spends the lowest cards.
pub struct RuleCoverage {
    priority: CoveragePriority,
}

impl RuleCoverage {
    pub fn new(priority: CoveragePriority) -> Self {
        RuleCoverage { priority }
    }
}

/// My palette after `mv`.
fn palette_after(view: &PlayerView, mv: Move) -> CardSet {
    let palette = view.palette(view.me());
    match mv {
        Move::Palette(card) | Move::Both { palette: card, .. } => palette | card.bit(),
        Move::Canvas(_) => palette,
    }
}

/// How many rules I'd be winning under with `palette`.
fn rule_coverage(view: &PlayerView, palette: CardSet) -> usize {
    Color::ALL
        .iter()
        .filter(|&&rule| view.would_win(palette, rule))
        .count()
}

/// (number of cards spent, sum of their ids). A lower id sum means weaker
/// cards were given up.
fn cards_spent(mv: Move) -> (u32, u32) {
    match mv {
        Move::Palette(card) | Move::Canvas(card) => (1, u32::from(card.id())),
        Move::Both { palette, canvas } => (2, u32::from(palette.id()) + u32::from(canvas.id())),
    }
}

impl Strategy for RuleCoverage {
    fn name(&self) -> &str {
        match self.priority {
            CoveragePriority::CoverageFirst => "rule-coverage",
            CoveragePriority::CardsFirst => "rule-coverage-economy",
        }
    }
    fn choose(&self, view: &PlayerView, legal: &[Move], _rng: &mut dyn Rng) -> Move {
        legal
            .iter()
            .copied()
            .min_by_key(|&mv| {
                // Minimizing rules *not* led = maximizing coverage.
                let rules_not_led = Color::ALL.len() - rule_coverage(view, palette_after(view, mv));
                let (count, id_sum) = cards_spent(mv);
                match self.priority {
                    CoveragePriority::CoverageFirst => (rules_not_led as u32, count, id_sum),
                    CoveragePriority::CardsFirst => (count, rules_not_led as u32, id_sum),
                }
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
    use crate::card::{Card, Color};
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

    // In `pick`'s fresh 2-player state both palettes are empty, so a move's
    // coverage is the number of rules under which the new palette scores > 0.
    // R7 alone: 5 (fails Green and Violet). G2 alone: 7. Canvas-only: 0.

    #[test]
    fn coverage_first_spends_two_cards_for_more_coverage() {
        let low = Card::new(1, Color::Violet);
        let red_seven = Card::new(7, Color::Red);
        let green_two = Card::new(2, Color::Green);
        let legal = [
            Move::Canvas(low),
            Move::Palette(red_seven),
            Move::Both {
                palette: green_two,
                canvas: low,
            },
        ];
        assert_eq!(
            pick(&RuleCoverage::new(CoveragePriority::CoverageFirst), &legal),
            Move::Both {
                palette: green_two,
                canvas: low
            }
        );
        assert_eq!(
            pick(&RuleCoverage::new(CoveragePriority::CardsFirst), &legal),
            Move::Palette(red_seven)
        );
    }

    #[test]
    fn coverage_tie_spends_lowest_card() {
        let red_two = Card::new(2, Color::Red);
        let green_two = Card::new(2, Color::Green);
        let legal = [Move::Palette(red_two), Move::Palette(green_two)];
        for priority in [
            CoveragePriority::CoverageFirst,
            CoveragePriority::CardsFirst,
        ] {
            assert_eq!(
                pick(&RuleCoverage::new(priority), &legal),
                Move::Palette(green_two)
            );
        }
    }

    #[test]
    fn aggressive_prefers_palette_and_plays_highest() {
        let low = Card::new(2, Color::Violet);
        let high = Card::new(6, Color::Red);
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
        let low = Card::new(3, Color::Blue);
        let high = Card::new(3, Color::Orange);
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
        let a = Card::new(1, Color::Green);
        let b = Card::new(4, Color::Yellow);
        let c = Card::new(7, Color::Indigo);
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
