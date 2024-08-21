use std::fmt::Debug;
use std::hash::Hash;

mod ordered_set;
pub use ordered_set::OrderedSet;

pub trait Ordered: Debug + PartialEq + Eq + PartialOrd + Ord + Clone + Hash {}
impl<T> Ordered for T where T: Debug + PartialEq + Eq + PartialOrd + Ord + Clone + Hash {}

macro_rules! make_set_traits {
    ($($x:ty),+) => {$(
        impl<T: Ordered> std::ops::BitAnd<&$x> for &$x {
            type Output = $x;

            fn bitand(self, other: &$x) -> Self::Output {
                (&self.0 & &other.0).into()
            }
        }

        impl<T: Ordered> std::ops::BitAndAssign<&$x> for $x {
            fn bitand_assign(&mut self, other: &$x) {
                self.0 = &self.0 & &other.0;
            }
        }

        impl<T: Ordered> std::ops::BitAndAssign<$x> for $x {
            fn bitand_assign(&mut self, other: $x) {
                self.0 = &self.0 & &other.0;
            }
        }

        impl<T: Ordered> std::ops::BitOr<&$x> for &$x {
            type Output = $x;

            fn bitor(self, other: &$x) -> Self::Output {
                (&self.0 | &other.0).into()
            }
        }

        impl<T: Ordered> std::ops::BitOrAssign<&$x> for $x {
            fn bitor_assign(&mut self, other: &$x) {
                self.0 = &self.0 | &other.0;
            }
        }

        impl<T: Ordered> std::ops::BitOrAssign<$x> for $x {
            fn bitor_assign(&mut self, other: $x) {
                self.0 = &self.0 | &other.0;
            }
        }

        impl<T: Ordered> std::ops::BitXor<&$x> for &$x {
            type Output = $x;

            fn bitxor(self, other: &$x) -> Self::Output {
                (&self.0 ^ &other.0).into()
            }
        }

        impl<T: Ordered> std::ops::BitXorAssign<&$x> for $x {
            fn bitxor_assign(&mut self, other: &$x) {
                self.0 = &self.0 ^ &other.0;
            }
        }

        impl<T: Ordered> std::ops::BitXorAssign<$x> for $x {
            fn bitxor_assign(&mut self, other: $x) {
                self.0 = &self.0 ^ &other.0;
            }
        }

        impl<T: Ordered> std::ops::Sub<&$x> for &$x {
            type Output = $x;

            fn sub(self, other: &$x) -> Self::Output {
                (&self.0 - &other.0).into()
            }
        }

        impl<T: Ordered> std::ops::SubAssign<&$x> for $x {
            fn sub_assign(&mut self, other: &$x) {
                self.0 = &self.0 - &other.0;
            }
        }

        impl<T: Ordered> std::ops::SubAssign<$x> for $x {
            fn sub_assign(&mut self, other: $x) {
                self.0 = &self.0 - &other.0;
            }
        }
    )+};
}
use make_set_traits;
