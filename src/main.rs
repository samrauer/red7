use std::time::Instant;

use rand::rngs::SmallRng;
use rand::{RngExt, SeedableRng};
use rayon::prelude::*;

use red7::game::{MAX_PLAYERS, MIN_PLAYERS, play_game};
use red7::strategy::{self, Strategy};

const SEED: u64 = 0x5EED;

#[derive(Clone, Copy, Default)]
struct Tally {
    seats: u64,
    wins: u64,
}

/// Results indexed by player count (`0..MIN_PLAYERS` unused).
#[derive(Clone)]
struct Stats {
    games: [u64; MAX_PLAYERS + 1],
    /// `by_strategy[s][n]`: strategy `s`'s seats and wins in `n`-player games.
    by_strategy: Vec<[Tally; MAX_PLAYERS + 1]>,
}

impl Stats {
    fn new(num_strategies: usize) -> Self {
        Stats {
            games: [0; MAX_PLAYERS + 1],
            by_strategy: vec![[Tally::default(); MAX_PLAYERS + 1]; num_strategies],
        }
    }

    fn merge(mut self, other: Self) -> Self {
        for (g, o) in self.games.iter_mut().zip(other.games) {
            *g += o;
        }
        for (mine, theirs) in self.by_strategy.iter_mut().zip(other.by_strategy) {
            for (t, u) in mine.iter_mut().zip(theirs) {
                t.seats += u.seats;
                t.wins += u.wins;
            }
        }
        self
    }
}

/// Play `games` games. Each game picks a random player count and gives every
/// seat an independently, uniformly chosen strategy from `bots`.
fn simulate(bots: &[&dyn Strategy], games: u64, seed: u64) -> Stats {
    (0..games)
        .into_par_iter()
        .fold(
            || Stats::new(bots.len()),
            |mut acc, i| {
                // Per-game seed => results are reproducible regardless of threading.
                let mut rng = SmallRng::seed_from_u64(seed.wrapping_add(i));
                let n = rng.random_range(MIN_PLAYERS..=MAX_PLAYERS);
                let picks: Vec<usize> = (0..n).map(|_| rng.random_range(0..bots.len())).collect();
                let seats: Vec<&dyn Strategy> = picks.iter().map(|&s| bots[s]).collect();
                let winner = play_game(&seats, &mut rng);

                acc.games[n] += 1;
                for &s in &picks {
                    acc.by_strategy[s][n].seats += 1;
                }
                acc.by_strategy[picks[winner]][n].wins += 1;
                acc
            },
        )
        .reduce(|| Stats::new(bots.len()), Stats::merge)
}

fn main() {
    let games: u64 = std::env::args()
        .nth(1)
        .map(|s| s.parse().expect("usage: red7 [games]"))
        .unwrap_or(100_000);

    let monte_carlo_vs_random = strategy::MonteCarlo::new(50, &strategy::Random);
    let monte_carlo_vs_conservative = strategy::MonteCarlo::new(50, &strategy::ConservativePalette);
    let bots: Vec<&dyn Strategy> = vec![
        &strategy::Random,
        &strategy::ConservativePalette,
        &strategy::AggressivePalette,
        &monte_carlo_vs_random,
        &monte_carlo_vs_conservative,
    ];

    let start = Instant::now();
    let stats = simulate(&bots, games, SEED);

    println!("{games} games ({MIN_PLAYERS}-{MAX_PLAYERS} players, each seat a random strategy)\n");

    // Columns: all player counts combined, then each player count.
    let columns: Vec<(String, Vec<usize>)> =
        std::iter::once(("all".to_string(), (MIN_PLAYERS..=MAX_PLAYERS).collect()))
            .chain((MIN_PLAYERS..=MAX_PLAYERS).map(|n| (format!("{n}p"), vec![n])))
            .collect();

    let width = bots
        .iter()
        .map(|b| b.name().len())
        .max()
        .unwrap_or(0)
        .max(8);
    let pct = |num: u64, den: u64| {
        if den == 0 {
            "-".to_string()
        } else {
            format!("{:.2}%", 100.0 * num as f64 / den as f64)
        }
    };
    let row = |label: &str, cells: Vec<String>| {
        print!("{label:<width$}");
        for c in cells {
            print!(" {c:>8}");
        }
        println!();
    };

    println!("win rate per seat");
    row("", columns.iter().map(|(name, _)| name.clone()).collect());
    for (s, bot) in bots.iter().enumerate() {
        let t = &stats.by_strategy[s];
        row(
            bot.name(),
            columns
                .iter()
                .map(|(_, ns)| {
                    let wins = ns.iter().map(|&n| t[n].wins).sum();
                    let seats = ns.iter().map(|&n| t[n].seats).sum();
                    pct(wins, seats)
                })
                .collect(),
        );
    }
    row(
        "fair",
        columns
            .iter()
            .map(|(_, ns)| {
                let games = ns.iter().map(|&n| stats.games[n]).sum();
                let seats = ns.iter().map(|&n| stats.games[n] * n as u64).sum();
                pct(games, seats)
            })
            .collect(),
    );
    row(
        "games",
        columns
            .iter()
            .map(|(_, ns)| ns.iter().map(|&n| stats.games[n]).sum::<u64>().to_string())
            .collect(),
    );

    println!("\ndone in {:.2?}", start.elapsed());
}
