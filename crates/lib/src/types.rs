use std::fmt::Debug;
use std::hash::Hash;

mod ordered_set;
pub use ordered_set::OrderedSet;

pub trait Ordered: Debug + PartialEq + Eq + PartialOrd + Ord + Clone + Hash {}
impl<T> Ordered for T where T: Debug + PartialEq + Eq + PartialOrd + Ord + Clone + Hash {}

macro_rules! make_set_traits {
    ($($x:ty),+) => {$(
        impl<T: Ordered> BitAnd<&$x> for &$x {
            type Output = $x;

            fn bitand(self, other: &$x) -> Self::Output {
                (&self.0 & &other.0).into()
            }
        }

        impl<T: Ordered> BitAndAssign<&$x> for $x {
            fn bitand_assign(&mut self, other: &$x) {
                self.0 = &self.0 & &other.0;
            }
        }

        impl<T: Ordered> BitAndAssign<$x> for $x {
            fn bitand_assign(&mut self, other: $x) {
                self.0 = &self.0 & &other.0;
            }
        }

        impl<T: Ordered> BitOr<&$x> for &$x {
            type Output = $x;

            fn bitor(self, other: &$x) -> Self::Output {
                (&self.0 | &other.0).into()
            }
        }

        impl<T: Ordered> BitOrAssign<&$x> for $x {
            fn bitor_assign(&mut self, other: &$x) {
                self.0 = &self.0 | &other.0;
            }
        }

        impl<T: Ordered> BitOrAssign<$x> for $x {
            fn bitor_assign(&mut self, other: $x) {
                self.0 = &self.0 | &other.0;
            }
        }

        impl<T: Ordered> BitXor<&$x> for &$x {
            type Output = $x;

            fn bitxor(self, other: &$x) -> Self::Output {
                (&self.0 ^ &other.0).into()
            }
        }

        impl<T: Ordered> BitXorAssign<&$x> for $x {
            fn bitxor_assign(&mut self, other: &$x) {
                self.0 = &self.0 ^ &other.0;
            }
        }

        impl<T: Ordered> BitXorAssign<$x> for $x {
            fn bitxor_assign(&mut self, other: $x) {
                self.0 = &self.0 ^ &other.0;
            }
        }

        impl<T: Ordered> Sub<&$x> for &$x {
            type Output = $x;

            fn sub(self, other: &$x) -> Self::Output {
                (&self.0 - &other.0).into()
            }
        }

        impl<T: Ordered> SubAssign<&$x> for $x {
            fn sub_assign(&mut self, other: &$x) {
                self.0 = &self.0 - &other.0;
            }
        }

        impl<T: Ordered> SubAssign<$x> for $x {
            fn sub_assign(&mut self, other: $x) {
                self.0 = &self.0 - &other.0;
            }
        }
    )+};
}
use make_set_traits;
