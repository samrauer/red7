//! Scoring a palette under each canvas rule.
//!
//! `score` returns a single `u16` so that comparing two palettes is one
//! integer comparison: `(qualifying card count) << COUNT_SHIFT | (highest qualifying card + 1)`.
//! The count dominates; the highest qualifying card breaks ties, exactly as in
//! the official rules. A score of 0 means the palette has no qualifying cards,
//! and such a palette can never be "winning".

use crate::card::{CardSet, Color};

/// `NUMBER_MASKS[i]`: all seven cards with number `i + 1`.
const NUMBER_MASKS: [CardSet; 7] = {
    let mut masks = [CardSet::EMPTY; 7];
    let mut i = 0;
    while i < 7 {
        masks[i] = CardSet::all_of_number(i as u8 + 1);
        i += 1;
    }
    masks
};

/// `COLOR_MASKS[i]`: all seven cards of color `i`.
const COLOR_MASKS: [CardSet; 7] = {
    let mut masks = [CardSet::EMPTY; 7];
    let mut i = 0;
    while i < 7 {
        masks[i] = CardSet::all_of_color(Color::ALL[i]);
        i += 1;
    }
    masks
};

/// Numbers 2, 4, 6 (indices 1, 3, 5).
const EVEN_MASK: CardSet = NUMBER_MASKS[1]
    .union(NUMBER_MASKS[3])
    .union(NUMBER_MASKS[5]);
/// Numbers 1, 2, 3.
const BELOW_4_MASK: CardSet = NUMBER_MASKS[0]
    .union(NUMBER_MASKS[1])
    .union(NUMBER_MASKS[2]);

/// The qualifying-card count sits above this bit; below it is the tie-breaker
/// (highest card + 1, at most 49, which fits in 6 bits).
pub const COUNT_SHIFT: u32 = 6;

/// Score with `count` qualifying cards, tie-broken by the highest card in
/// `qualifying`. Returns 0 if `qualifying` is empty.
#[inline]
fn encode(count: u32, qualifying: CardSet) -> u16 {
    qualifying.highest().map_or(0, |t| {
        ((count as u16) << COUNT_SHIFT) | (u16::from(t.id()) + 1)
    })
}

/// Score of a qualifying subset where every card counts.
#[inline]
fn subset_score(set: CardSet) -> u16 {
    encode(set.len() as u32, set)
}

pub fn score(palette: CardSet, rule: Color) -> u16 {
    match rule {
        Color::Red => encode(1, palette),
        Color::Orange => NUMBER_MASKS
            .iter()
            .map(|&m| subset_score(palette & m))
            .max()
            .unwrap(),
        Color::Yellow => COLOR_MASKS
            .iter()
            .map(|&m| subset_score(palette & m))
            .max()
            .unwrap(),
        Color::Green => subset_score(palette & EVEN_MASK),
        Color::Violet => subset_score(palette & BELOW_4_MASK),
        Color::Blue => {
            // One card per distinct color counts; the best tie-breaker is
            // simply the highest card overall (it is the top of its color).
            let colors = COLOR_MASKS
                .iter()
                .filter(|&&m| palette.intersects(m))
                .count() as u32;
            encode(colors, palette)
        }
        Color::Indigo => {
            // Longest run of consecutive numbers; ties broken by the highest
            // card in the run (= highest card of the run's top number).
            let mut best = 0u16;
            let mut run_len = 0u32;
            for &mask in &NUMBER_MASKS {
                let at_n = palette & mask;
                if at_n.is_empty() {
                    run_len = 0;
                } else {
                    run_len += 1;
                    best = best.max(encode(run_len, at_n));
                }
            }
            best
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{Card, Color::*};

    fn set(cards: &[(u8, Color)]) -> CardSet {
        cards.iter().map(|&(n, col)| Card::new(n, col)).collect()
    }

    #[test]
    fn red_highest_card() {
        let a = set(&[(7, Violet), (1, Red)]);
        let b = set(&[(6, Red), (5, Red)]);
        assert!(score(a, Red) > score(b, Red));
    }

    #[test]
    fn orange_count_then_tiebreak() {
        let pair_of_threes = set(&[(3, Red), (3, Blue)]);
        let pair_of_twos = set(&[(2, Red), (2, Blue), (7, Red)]);
        // Equal counts (2 vs 2): highest qualifying card is R3 vs R2.
        assert!(score(pair_of_threes, Orange) > score(pair_of_twos, Orange));
    }

    #[test]
    fn indigo_run() {
        let run3 = set(&[(1, Red), (2, Violet), (3, Violet)]);
        let run2 = set(&[(6, Red), (7, Red)]);
        assert!(score(run3, Indigo) > score(run2, Indigo));
        assert_eq!(score(run3, Indigo) >> COUNT_SHIFT, 3);
    }

    #[test]
    fn violet_and_green_can_be_zero() {
        let high_odd = set(&[(5, Red), (7, Blue)]);
        assert_eq!(score(high_odd, Violet), 0);
        assert_eq!(score(high_odd, Green), 0);
    }

    #[test]
    fn blue_distinct_colors() {
        let three_colors = set(&[(1, Red), (1, Blue), (1, Green)]);
        let same_color = set(&[(7, Red), (6, Red), (5, Red), (4, Red)]);
        assert!(score(three_colors, Blue) > score(same_color, Blue));
    }
}
