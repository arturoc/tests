use std::ops::BitOr;
use std::usize;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Bitmask{
    pub has: usize,
    pub not: usize,
}

impl Bitmask{
    #[inline]
    pub fn has(has: usize) -> Bitmask {
        Bitmask{
            has,
            not: usize::MAX
        }
    }

    #[inline]
    pub fn not(not: usize) -> Bitmask {
        Bitmask{
            has: 0,
            not,
        }
    }

    #[inline]
    pub fn has_not(has: usize, not: usize) -> Bitmask {
        Bitmask{
            has,
            not,
        }
    }

    #[inline]
    pub fn check(&self, mask: usize) -> bool {
        mask & self.has == self.has && mask & self.not != self.not
    }
}

impl BitOr for Bitmask{
    type Output = Bitmask;
    fn bitor(self, rhs: Bitmask) -> Bitmask {
        let has = self.has | rhs.has;
        let not = self.not & rhs.not;
        Bitmask{has, not}
    }
}