use std::fmt;
use std::marker::PhantomData;

mod roles {
    pub(crate) enum Certain {}
    pub(crate) enum Maybe {}
    pub(crate) enum Own {}
    pub(crate) enum Outer {}
    pub(crate) enum Touched {}
    pub(crate) enum Row {}
    pub(crate) enum Hidden {}
    pub(crate) enum Reads {}
}

/// Bound in every row.
pub(crate) type Certain = Slots<roles::Certain>;
/// Bound in some row, which includes what is bound in every row.
pub(crate) type Maybe = Slots<roles::Maybe>;
/// Bound in every row by a group's own patterns, as it would bind them alone.
pub(crate) type Own = Slots<roles::Own>;
/// Bound, perhaps, by the groups around a group, which it does not see.
pub(crate) type Outer = Slots<roles::Outer>;
/// Bound, in some row or every row, by a group's own patterns.
pub(crate) type Touched = Slots<roles::Touched>;
/// Bound, perhaps, in the row an EXISTS body runs on.
pub(crate) type Row = Slots<roles::Row>;
/// Read as unbound by a group although a group around it binds it.
pub(crate) type Hidden = Slots<roles::Hidden>;
/// Mentioned by a pattern or read by an expression.
pub(crate) type Reads = Slots<roles::Reads>;

const WORD: usize = u64::BITS as usize;

/// A set of slots tagged with the role it plays, so that a set of one role cannot stand
/// for another. Slots are small and dense, which a bit for each keeps cheap to copy.
pub(crate) struct Slots<Role> {
    words: Vec<u64>,
    role: PhantomData<Role>,
}

impl<Role> Slots<Role> {
    pub(crate) fn new() -> Self {
        Self {
            words: Vec::new(),
            role: PhantomData,
        }
    }

    pub(crate) fn contains(&self, slot: usize) -> bool {
        self.words
            .get(slot / WORD)
            .is_some_and(|word| (word >> (slot % WORD)) & 1 == 1)
    }

    pub(crate) fn insert(&mut self, slot: usize) {
        if self.words.len() <= slot / WORD {
            self.words.resize(slot / WORD + 1, 0);
        }
        self.words[slot / WORD] |= 1 << (slot % WORD);
    }

    /// The slots in ascending order.
    pub(crate) fn iter(&self) -> impl Iterator<Item = usize> + '_ {
        self.words.iter().enumerate().flat_map(|(at, &word)| {
            let mut left = word;
            std::iter::from_fn(move || {
                (left != 0).then(|| {
                    let bit = left.trailing_zeros() as usize;
                    left &= left - 1;
                    at * WORD + bit
                })
            })
        })
    }

    pub(crate) fn intersection(&self, other: &Self) -> Self {
        Self {
            words: self
                .words
                .iter()
                .zip(&other.words)
                .map(|(a, b)| a & b)
                .collect(),
            role: PhantomData,
        }
    }

    pub(crate) fn is_subset_of<Other>(&self, other: &Slots<Other>) -> bool {
        self.words
            .iter()
            .enumerate()
            .all(|(at, word)| (word & !other.words.get(at).copied().unwrap_or(0)) == 0)
    }

    pub(crate) fn is_disjoint_from<Other>(&self, other: &Slots<Other>) -> bool {
        self.words.iter().zip(&other.words).all(|(a, b)| a & b == 0)
    }
}

impl<Role> Default for Slots<Role> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Role> Clone for Slots<Role> {
    fn clone(&self) -> Self {
        Self {
            words: self.words.clone(),
            role: PhantomData,
        }
    }
}

impl<Role> fmt::Debug for Slots<Role> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_set().entries(self.iter()).finish()
    }
}

impl<Role> FromIterator<usize> for Slots<Role> {
    fn from_iter<T: IntoIterator<Item = usize>>(slots: T) -> Self {
        let mut set = Self::new();
        set.extend(slots);
        set
    }
}

impl<Role> Extend<usize> for Slots<Role> {
    fn extend<T: IntoIterator<Item = usize>>(&mut self, slots: T) {
        for slot in slots {
            self.insert(slot);
        }
    }
}

impl<Role> IntoIterator for Slots<Role> {
    type Item = usize;
    type IntoIter = Taken;

    fn into_iter(self) -> Taken {
        Taken {
            words: self.words,
            at: 0,
            left: 0,
        }
    }
}

/// The slots of a set that is given up, in ascending order.
pub(crate) struct Taken {
    words: Vec<u64>,
    at: usize,
    left: u64,
}

impl Iterator for Taken {
    type Item = usize;

    fn next(&mut self) -> Option<usize> {
        while self.left == 0 {
            self.left = *self.words.get(self.at)?;
            self.at += 1;
        }
        let bit = self.left.trailing_zeros() as usize;
        self.left &= self.left - 1;
        Some((self.at - 1) * WORD + bit)
    }
}
