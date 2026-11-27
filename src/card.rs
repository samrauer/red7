//! Card representation.
//!
//! The 49 cards are encoded as `u8` ids in `0..49` such that a higher id is
//! always a stronger card: `id = (number - 1) * 7 + color`, where colors are
//! ordered Violet (weakest) .. Red (strongest). Comparing two cards is
//! therefore just comparing their ids.
//!
//! A set of cards (hand, palette) is a `CardSet`: a `u64` bitmask with bit
//! `id` set if that card is present.

use std::fmt;
use std::ops::{BitAnd, BitOr, BitOrAssign, Sub};

/// A single card, stored as its id.
/// Ordering compares ids, so a stronger card always compares greater.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Card(u8);

/// A set of cards.
/// Bit `id` is set if the card with that id is in the set.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct CardSet(u64);

pub const NUM_CARDS: u8 = 49;
pub const FULL_DECK: CardSet = CardSet((1u64 << NUM_CARDS) - 1);

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

    /// This card's bit in a `CardSet` bitmask.
    #[inline]
    fn bit(self) -> u64 {
        1u64 << self.0
    }
}

impl CardSet {
    pub const EMPTY: CardSet = CardSet(0);

    /// All seven cards with the given number.
    pub const fn all_of_number(number: u8) -> CardSet {
        debug_assert!(matches!(number, 1..=7));
        CardSet(0b111_1111 << ((number - 1) * 7))
    }

    /// All seven cards of the given color.
    pub const fn all_of_color(color: Color) -> CardSet {
        let mut bits = 0u64;
        let mut number = 0;
        while number < 7 {
            bits |= 1 << (number * 7 + color as u8);
            number += 1;
        }
        CardSet(bits)
    }

    /// The cards in either set. Same as `|`, but usable in `const` items.
    pub const fn union(self, other: CardSet) -> CardSet {
        CardSet(self.0 | other.0)
    }

    #[inline]
    pub fn contains(self, card: Card) -> bool {
        self.0 & card.bit() != 0
    }

    #[inline]
    pub fn insert(&mut self, card: Card) {
        self.0 |= card.bit();
    }

    #[inline]
    pub fn remove(&mut self, card: Card) {
        self.0 &= !card.bit();
    }

    /// A copy of this set with `card` added.
    #[inline]
    pub fn with(self, card: Card) -> CardSet {
        CardSet(self.0 | card.bit())
    }

    /// A copy of this set with `card` removed.
    #[inline]
    pub fn without(self, card: Card) -> CardSet {
        CardSet(self.0 & !card.bit())
    }

    /// Number of cards in the set.
    #[inline]
    pub fn len(self) -> usize {
        self.0.count_ones() as usize
    }

    #[inline]
    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Do the two sets have any card in common?
    #[inline]
    pub fn intersects(self, other: CardSet) -> bool {
        self.0 & other.0 != 0
    }

    /// Highest card in the set, if any.
    #[inline]
    pub fn highest(self) -> Option<Card> {
        self.0.checked_ilog2().map(|id| Card::from_id(id as u8))
    }

    /// Iterate cards in the set, lowest first.
    #[inline]
    pub fn cards(self) -> impl Iterator<Item = Card> {
        let mut remaining = self.0;
        std::iter::from_fn(move || {
            if remaining == 0 {
                None
            } else {
                let id = remaining.trailing_zeros() as u8;
                remaining &= remaining - 1;
                Some(Card::from_id(id))
            }
        })
    }
}

/// Union: the cards in either set.
impl BitOr for CardSet {
    type Output = CardSet;
    #[inline]
    fn bitor(self, other: CardSet) -> CardSet {
        CardSet(self.0 | other.0)
    }
}

impl BitOrAssign for CardSet {
    #[inline]
    fn bitor_assign(&mut self, other: CardSet) {
        self.0 |= other.0;
    }
}

/// Intersection: the cards in both sets.
impl BitAnd for CardSet {
    type Output = CardSet;
    #[inline]
    fn bitand(self, other: CardSet) -> CardSet {
        CardSet(self.0 & other.0)
    }
}

/// Set difference: the cards in `self` that are not in `other`.
impl Sub for CardSet {
    type Output = CardSet;
    // Set difference is a mask operation, which this lint mistakes for a bug.
    #[allow(clippy::suspicious_arithmetic_impl)]
    #[inline]
    fn sub(self, other: CardSet) -> CardSet {
        CardSet(self.0 & !other.0)
    }
}

/// The set containing just this card.
impl From<Card> for CardSet {
    #[inline]
    fn from(card: Card) -> CardSet {
        CardSet(card.bit())
    }
}

impl FromIterator<Card> for CardSet {
    fn from_iter<I: IntoIterator<Item = Card>>(cards: I) -> CardSet {
        let mut set = CardSet::EMPTY;
        for card in cards {
            set.insert(card);
        }
        set
    }
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

/// Prints a card set as e.g. `[B2 G5 R7]`, lowest card first.
impl fmt::Display for CardSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let names: Vec<String> = self.cards().map(|card| card.to_string()).collect();
        write!(f, "[{}]", names.join(" "))
    }
}

/// Same as `Display`, so test failures show cards rather than a bitmask.
impl fmt::Debug for CardSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
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
        for c in FULL_DECK.cards() {
            assert_eq!(Card::new(c.number(), c.color()), c);
        }
    }

    #[test]
    fn set_operations() {
        let b2 = Card::new(2, Color::Blue);
        let r7 = Card::new(7, Color::Red);
        let mut set = CardSet::EMPTY;
        assert!(set.is_empty());

        set.insert(r7);
        set.insert(b2);
        assert_eq!(set.len(), 2);
        assert!(set.contains(b2) && set.contains(r7));
        assert_eq!(set.highest(), Some(r7));
        assert_eq!(format!("{set:?}"), "[B2 R7]");
        assert_eq!(set.without(r7), CardSet::from(b2));
        assert_eq!(FULL_DECK - set, FULL_DECK.without(b2).without(r7));

        set.remove(b2);
        assert_eq!(set, CardSet::from(r7));
    }

    #[test]
    fn number_and_color_masks() {
        for number in 1..=7 {
            let set = CardSet::all_of_number(number);
            assert_eq!(set.len(), 7);
            assert!(set.cards().all(|c| c.number() == number));
        }
        for color in Color::ALL {
            let set = CardSet::all_of_color(color);
            assert_eq!(set.len(), 7);
            assert!(set.cards().all(|c| c.color() == color));
        }
    }
}
