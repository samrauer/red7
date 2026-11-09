use std::ops::AddAssign;
use std::time::Instant;

use rand::rngs::SmallRng;
use rand::{RngExt, SeedableRng};
use rayon::prelude::*;

use red7::game::{MAX_PLAYERS, MIN_PLAYERS, play_game};
use red7::strategy::{self, Strategy};

const SEED: u64 = 0x5EED;

#[derive(Clone, Copy, Default)]
struct Tally {
    games: u64,
    a_seats: u64,
    b_seats: u64,
    a_wins: u64,
    b_wins: u64,
}

impl AddAssign for Tally {
    fn add_assign(&mut self, o: Self) {
        self.games += o.games;
        self.a_seats += o.a_seats;
        self.b_seats += o.b_seats;
        self.a_wins += o.a_wins;
        self.b_wins += o.b_wins;
    }
}

/// Tallies indexed by player count.
type ByPlayers = [Tally; MAX_PLAYERS + 1];

/// Play `games` games between `a` and `b`. Each game picks a random player
/// count and fills every seat with `a` or `b` at random (at least one of each).
fn matchup(a: &dyn Strategy, b: &dyn Strategy, games: u64, seed: u64) -> ByPlayers {
    (0..games)
        .into_par_iter()
        .fold(ByPlayers::default, |mut acc, i| {
            // Per-game seed => results are reproducible regardless of threading.
            let mut rng = SmallRng::seed_from_u64(seed.wrapping_add(i));
            let n = rng.random_range(MIN_PLAYERS..=MAX_PLAYERS);
            let is_a: Vec<bool> = loop {
                let s: Vec<bool> = (0..n).map(|_| rng.random_bool(0.5)).collect();
                if s.contains(&true) && s.contains(&false) {
                    break s;
                }
            };
            let seats: Vec<&dyn Strategy> = is_a.iter().map(|&x| if x { a } else { b }).collect();
            let winner = play_game(&seats, &mut rng);

            let a_seats = is_a.iter().filter(|&&x| x).count() as u64;
            let a_won = u64::from(is_a[winner]);
            acc[n] += Tally {
                games: 1,
                a_seats,
                b_seats: n as u64 - a_seats,
                a_wins: a_won,
                b_wins: 1 - a_won,
            };
            acc
        })
        .reduce(ByPlayers::default, |mut x, y| {
            for (t, u) in x.iter_mut().zip(y) {
                *t += u;
            }
            x
        })
}

fn print_row(label: &str, t: &Tally) {
    let pct = |num: u64, den: u64| 100.0 * num as f64 / den as f64;
    println!(
        "  {label:<8} {:>9} games   {:>6.2}% / {:>6.2}% per seat   (fair {:>6.2}%)",
        t.games,
        pct(t.a_wins, t.a_seats),
        pct(t.b_wins, t.b_seats),
        pct(t.games, t.a_seats + t.b_seats),
    );
}

fn main() {
    let games: u64 = std::env::args()
        .nth(1)
        .map(|s| s.parse().expect("usage: red7 [games]"))
        .unwrap_or(100_000);

    let bots: Vec<&dyn Strategy> = vec![&strategy::Random, &strategy::Conservative];

    let start = Instant::now();
    println!(
        "{games} games per matchup ({MIN_PLAYERS}-{MAX_PLAYERS} players per game, seats randomly assigned)\n"
    );
    for (i, a) in bots.iter().enumerate() {
        for b in &bots[i + 1..] {
            let by_n = matchup(*a, *b, games, SEED);
            let mut total = Tally::default();
            for t in &by_n {
                total += *t;
            }
            println!("{} vs {}", a.name(), b.name());
            print_row("all", &total);
            for (n, t) in by_n.iter().enumerate().skip(MIN_PLAYERS) {
                print_row(&format!("{n}p"), t);
            }
            println!();
        }
    }
    println!("done in {:.2?}", start.elapsed());
}
