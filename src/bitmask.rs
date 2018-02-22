use std::ops::BitOr;
use std::usize;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Bitmask{
    Has(usize),
    Not(usize),
    HasNot(usize, usize)
}

impl Bitmask{
    pub fn has(&self) -> Option<usize> {
        match *self{
            Bitmask::Has(mask) => Some(mask),
            Bitmask::Not(_) => None,
            Bitmask::HasNot(mask,_) => Some(mask),
        }
    }

    pub fn not(&self) -> Option<usize> {
        match *self{
            Bitmask::Has(_) => None,
            Bitmask::Not(mask) => Some(mask),
            Bitmask::HasNot(_,mask) => Some(mask),
        }
    }
}

impl BitOr for Bitmask{
    type Output = Bitmask;
    fn bitor(self, rhs: Bitmask) -> Bitmask {
        let has = self.has().unwrap_or(0) | rhs.has().unwrap_or(0);
        let not = self.not().unwrap_or(usize::MAX) & rhs.not().unwrap_or(usize::MAX);
        Bitmask::HasNot(has, not)
    }
}