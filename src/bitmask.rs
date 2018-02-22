use std::ops::BitOr;

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
        let has = self.has().and_then(|has| rhs.has().map(|rhs| has | rhs));
        let not = self.not().and_then(|has| rhs.not().map(|rhs| has & rhs));
        match (has, not) {
            (Some(has), None) => Bitmask::Has(has),
            (None, Some(not)) => Bitmask::Not(not),
            (Some(has), Some(not)) => Bitmask::HasNot(has, not),
            (None, None) => unimplemented!()
        }
    }
}