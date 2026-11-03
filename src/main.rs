use std::time::Instant;

use rand::SeedableRng;
use rand::rngs::SmallRng;
use rayon::prelude::*;

use red7::game::play_game;
use red7::strategy::{self, Strategy};

const SEED: u64 = 0x5EED;

/// Play `games` head-to-head games, swapping seats every game so neither
/// strategy benefits from turn order. Returns wins for `a`.
fn head_to_head(a: &dyn Strategy, b: &dyn Strategy, games: u64, seed: u64) -> u64 {
    (0..games)
        .into_par_iter()
        .map(|i| {
            // Per-game seed => results are reproducible regardless of threading.
            let mut rng = SmallRng::seed_from_u64(seed.wrapping_add(i));
            let a_first = i % 2 == 0;
            let seats: [&dyn Strategy; 2] = if a_first { [a, b] } else { [b, a] };
            let winner = play_game(&seats, &mut rng);
            u64::from((winner == 0) == a_first)
        })
        .sum()
}

fn main() {
    let games: u64 = std::env::args()
        .nth(1)
        .map(|s| s.parse().expect("usage: red7 [games]"))
        .unwrap_or(100_000);

    let bots: Vec<&dyn Strategy> = vec![&strategy::Random, &strategy::Conservative];

    let start = Instant::now();
    println!("{games} games per matchup (2 players, seats alternated)\n");
    for (i, a) in bots.iter().enumerate() {
        for b in &bots[i + 1..] {
            let wins = head_to_head(*a, *b, games, SEED);
            println!(
                "{:>14} vs {:<14} {:>6.2}% / {:>6.2}%",
                a.name(),
                b.name(),
                100.0 * wins as f64 / games as f64,
                100.0 * (games - wins) as f64 / games as f64,
            );
        }
    }
    println!("\ndone in {:.2?}", start.elapsed());
}
