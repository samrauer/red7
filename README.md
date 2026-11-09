# red7

Monte Carlo simulator for the card game Red7, used to compare strategies.

```sh
cargo test
cargo run --release -- 1000000   # games per matchup
```

- `src/card.rs`: card encoding (u8 ids, u64 bitsets; higher id = stronger card)
- `src/rules.rs`: scoring a palette under each canvas rule
- `src/game.rs`: game state, legal moves, turn loop (basic rules, one round, 2-7 players).
  House-rule setup: 7 cards each, no starting palette card, seat 0 goes first.
- `src/strategy.rs`: the `Strategy` trait and bots
- `src/main.rs`: parallel pairwise tournament; each game picks a random player count
  and assigns each seat to one of the two strategies at random. Reports win% per seat
  overall and per player count, against the fair share.
