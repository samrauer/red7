//! Card representation.
//!
//! The 49 cards are encoded as `u8` ids in `0..49` such that a higher id is
//! always a stronger card: `id = (number - 1) * 7 + color`, where colors are
//! ordered Violet (weakest) .. Red (strongest). Comparing two cards is
//! therefore just comparing their ids.
//!
//! A set of cards (hand, palette) is a `u64` bitmask, bit `id` set if present.

use std::fmt;

/// A single card, stored as its id.
/// Ordering compares ids, so a stronger card always compares greater.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Card(u8);

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

impl Card {
    pub fn new(number: u8, color: Color) -> Card {
        debug_assert!((1..=7).contains(&number));
        Card((number - 1) * 7 + color as u8)
    }

    fn from_id(id: u8) -> Card {
        debug_assert!(id < NUM_CARDS);
        Card(id)
    }

    /// The card's id in `0..NUM_CARDS`; a higher id is a stronger card.
    pub fn id(self) -> u8 {
        self.0
    }

    pub fn number(self) -> u8 {
        self.0 / 7 + 1
    }

    pub fn color(self) -> Color {
        Color::ALL[(self.0 % 7) as usize]
    }

    /// The set containing just this card.
    pub fn bit(self) -> CardSet {
        1u64 << self.0
    }
}

/// Highest card in the set, if any.
pub fn highest(set: CardSet) -> Option<Card> {
    set.checked_ilog2().map(|b| Card::from_id(b as u8))
}

/// Iterate cards in a set, lowest first.
pub fn cards(mut set: CardSet) -> impl Iterator<Item = Card> {
    std::iter::from_fn(move || {
        if set == 0 {
            None
        } else {
            let c = set.trailing_zeros() as u8;
            set &= set - 1;
            Some(Card::from_id(c))
        }
    })
}

/// Prints a card as e.g. `R7`.
impl fmt::Display for Card {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.color().letter(), self.number())
    }
}

/// Same as `Display`, so test failures show `R7` rather than an id.
impl fmt::Debug for Card {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

/// Pretty-printer for a card set, e.g. `[B2 G5 R7]`.
pub struct SetDisplay(pub CardSet);

impl fmt::Display for SetDisplay {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let parts: Vec<String> = cards(self.0).map(|c| c.to_string()).collect();
        write!(f, "[{}]", parts.join(" "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordering_number_then_color() {
        // Higher number beats any lower number.
        assert!(Card::new(2, Color::Violet) > Card::new(1, Color::Red));
        // Same number: color rank breaks the tie.
        assert!(Card::new(5, Color::Red) > Card::new(5, Color::Orange));
        assert_eq!(Card::new(7, Color::Red).id(), NUM_CARDS - 1);
    }

    #[test]
    fn roundtrip() {
        for c in cards(FULL_DECK) {
            assert_eq!(Card::new(c.number(), c.color()), c);
        }
    }
}
