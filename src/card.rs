//! Card representation.
//!
//! The 49 cards are encoded as `u8` ids in `0..49` such that a higher id is
//! always a stronger card: `id = (number - 1) * 7 + color`, where colors are
//! ordered Violet (weakest) .. Red (strongest). Comparing two cards is
//! therefore just comparing their ids.
//!
//! A set of cards (hand, palette) is a `u64` bitmask, bit `id` set if present.

use std::fmt;

pub type Card = u8;
pub type CardSet = u64;

pub const NUM_CARDS: u8 = 49;
pub const FULL_DECK: CardSet = (1u64 << NUM_CARDS) - 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum Color {
    Violet = 0, // most cards below 4
    Indigo,     // most cards in a run
    Blue,       // most different colors
    Green,      // most even cards
    Yellow,     // most cards of one color
    Orange,     // most cards of one number
    Red,        // highest card
}

impl Color {
    pub const ALL: [Color; 7] = [
        Color::Violet,
        Color::Indigo,
        Color::Blue,
        Color::Green,
        Color::Yellow,
        Color::Orange,
        Color::Red,
    ];

    pub fn letter(self) -> char {
        ['V', 'I', 'B', 'G', 'Y', 'O', 'R'][self as usize]
    }
}

pub fn make_card(number: u8, color: Color) -> Card {
    debug_assert!((1..=7).contains(&number));
    (number - 1) * 7 + color as u8
}

pub fn number(card: Card) -> u8 {
    card / 7 + 1
}

pub fn color(card: Card) -> Color {
    Color::ALL[(card % 7) as usize]
}

pub fn bit(card: Card) -> CardSet {
    1u64 << card
}

/// Highest card in the set, if any.
pub fn highest(set: CardSet) -> Option<Card> {
    set.checked_ilog2().map(|b| b as Card)
}

/// Iterate cards in a set, lowest first.
pub fn cards(mut set: CardSet) -> impl Iterator<Item = Card> {
    std::iter::from_fn(move || {
        if set == 0 {
            None
        } else {
            let c = set.trailing_zeros() as u8;
            set &= set - 1;
            Some(c)
        }
    })
}

/// Pretty-printer for a single card, e.g. `R7`.
pub struct CardDisplay(pub Card);

impl fmt::Display for CardDisplay {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", color(self.0).letter(), number(self.0))
    }
}

/// Pretty-printer for a card set, e.g. `[B2 G5 R7]`.
pub struct SetDisplay(pub CardSet);

impl fmt::Display for SetDisplay {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let parts: Vec<String> = cards(self.0).map(|c| CardDisplay(c).to_string()).collect();
        write!(f, "[{}]", parts.join(" "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordering_number_then_color() {
        // Higher number beats any lower number.
        assert!(make_card(2, Color::Violet) > make_card(1, Color::Red));
        // Same number: color rank breaks the tie.
        assert!(make_card(5, Color::Red) > make_card(5, Color::Orange));
        assert_eq!(make_card(7, Color::Red), NUM_CARDS - 1);
    }

    #[test]
    fn roundtrip() {
        for c in 0..NUM_CARDS {
            assert_eq!(make_card(number(c), color(c)), c);
        }
    }
}
