// TODO: This type can possibly be dropped if/when indexmap upstream implements an order-aware
// alternative type or changes IndexSet.
//
// See the following issues for more info:
// https://github.com/bluss/indexmap/issues/135
// https://github.com/bluss/indexmap/issues/153

use std::cmp::Ordering;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};
use std::ops::{
    BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Deref, DerefMut, Sub, SubAssign,
};

use indexmap::IndexSet;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub trait Ordered: Debug + PartialEq + Eq + PartialOrd + Ord + Clone + Hash {}
impl<T> Ordered for T where T: Debug + PartialEq + Eq + PartialOrd + Ord + Clone + Hash {}

/// Ordered set that implements Ord and Hash.
#[derive(Debug, Clone)]
pub struct OrderedSet<T: Ordered>(IndexSet<T>);

impl<T: Ordered> Default for OrderedSet<T> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<T: Ordered> OrderedSet<T> {
    /// Construct a new, empty OrderedSet<T>.
    pub fn new() -> Self {
        Self::default()
    }

    /// Return true if the set is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl<T: Ordered> Hash for OrderedSet<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for e in &self.0 {
            e.hash(state);
        }
    }
}

impl<T: Ordered> Ord for OrderedSet<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.iter().cmp(other.0.iter())
    }
}

impl<T: Ordered> PartialOrd for OrderedSet<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: Ordered> PartialEq for OrderedSet<T> {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl<T: Ordered> Eq for OrderedSet<T> {}

impl<T: Ordered> FromIterator<T> for OrderedSet<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iterable: I) -> Self {
        Self(iterable.into_iter().collect())
    }
}

impl<T: Ordered, const N: usize> From<[T; N]> for OrderedSet<T> {
    fn from(arr: [T; N]) -> Self {
        Self::from_iter(arr)
    }
}

impl<'a, T: Ordered> IntoIterator for &'a OrderedSet<T> {
    type Item = &'a T;
    type IntoIter = indexmap::set::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<T: Ordered> IntoIterator for OrderedSet<T> {
    type Item = T;
    type IntoIter = indexmap::set::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<T: Ordered> Deref for OrderedSet<T> {
    type Target = IndexSet<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: Ordered> DerefMut for OrderedSet<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'de, T> Deserialize<'de> for OrderedSet<T>
where
    T: Ordered + Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        IndexSet::deserialize(deserializer).map(OrderedSet)
    }
}

impl<T> Serialize for OrderedSet<T>
where
    T: Serialize + Ordered,
{
    fn serialize<Se>(&self, serializer: Se) -> Result<Se::Ok, Se::Error>
    where
        Se: Serializer,
    {
        serializer.collect_seq(self)
    }
}

macro_rules! make_set_traits {
    ($($x:ty),+) => {$(
        impl<T: Ordered> BitAnd<&Self> for $x {
            type Output = Self;

            fn bitand(mut self, other: &Self) -> Self::Output {
                self &= other;
                self
            }
        }

        impl<T: Ordered> BitAndAssign<&Self> for $x {
            fn bitand_assign(&mut self, other: &Self) {
                self.0 = &self.0 & &other.0;
            }
        }

        impl<T: Ordered> BitOr<&Self> for $x {
            type Output = Self;

            fn bitor(mut self, other: &Self) -> Self::Output {
                self |= other;
                self
            }
        }

        impl<T: Ordered> BitOrAssign<&Self> for $x {
            fn bitor_assign(&mut self, other: &Self) {
                self.0 = &self.0 | &other.0;
            }
        }

        impl<T: Ordered> BitXor<&Self> for $x {
            type Output = Self;

            fn bitxor(mut self, other: &Self) -> Self::Output {
                self ^= other;
                self
            }
        }

        impl<T: Ordered> BitXorAssign<&Self> for $x {
            fn bitxor_assign(&mut self, other: &Self) {
                self.0 = &self.0 ^ &other.0;
            }
        }

        impl<T: Ordered> Sub<&Self> for $x {
            type Output = Self;

            fn sub(mut self, other: &Self) -> Self::Output {
                self -= other;
                self
            }
        }

        impl<T: Ordered> SubAssign<&Self> for $x {
            fn sub_assign(&mut self, other: &Self) {
                self.0 = &self.0 - &other.0;
            }
        }
    )+};
}
use make_set_traits;
make_set_traits!(OrderedSet<T>);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordered_set() {
        // different elements
        let s1 = OrderedSet::from(["a"]);
        let s2 = OrderedSet::from(["b"]);
        assert_ne!(&s1, &s2);
        assert_ne!(OrderedSet::from([s1, s2]).len(), 1);

        // different ordering
        let s1 = OrderedSet::from(["a", "b"]);
        let s2 = OrderedSet::from(["b", "a"]);
        assert_ne!(&s1, &s2);
        assert_ne!(OrderedSet::from([s1, s2]).len(), 1);

        // similar ordering
        let s1 = OrderedSet::from(["a", "b"]);
        let s2 = OrderedSet::from(["a", "b"]);
        assert_eq!(&s1, &s2);
        assert_eq!(OrderedSet::from([s1, s2]).len(), 1);

        // matching elements
        let s1 = OrderedSet::from(["a", "b", "a"]);
        let s2 = OrderedSet::from(["a", "b", "b"]);
        assert_eq!(&s1, &s2);
        assert_eq!(OrderedSet::from([s1, s2]).len(), 1);
    }
}
