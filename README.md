# red7

Monte Carlo simulator for the card game Red7, used to compare strategies.

```sh
cargo test
cargo run --release -- 1000000   # games per matchup
```

- `src/card.rs`: card encoding (u8 ids, u64 bitsets; higher id = stronger card)
- `src/rules.rs`: scoring a palette under each canvas rule
- `src/game.rs`: game state, legal moves, turn loop (basic rules, one round)
- `src/strategy.rs`: the `Strategy` trait and bots
- `src/main.rs`: parallel head-to-head tournament
