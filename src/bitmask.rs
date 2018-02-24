use std::ops::BitOr;

#[cfg(not(feature = "bigint"))]
mod mask {
    use std::usize;
    use std::sync::atomic::AtomicUsize;
    use std::sync::atomic::Ordering;
    pub type MaskType = usize;

    pub struct NextMask{
        next: AtomicUsize,
    }

    impl NextMask {
        pub fn new() -> NextMask {
            NextMask{
                next: AtomicUsize::new(1)
            }
        }

        #[inline]
        pub fn next(&mut self) -> MaskType{
            let next = self.next.get_mut();
            if (*next as isize) < 0 {
                panic!("Trying to register more than 64 components, please use the bigint feature in your cargo dependency");
            }
            let ret = *next;
            *next *= 2;
            ret
        }
        
        pub fn get(&self) -> MaskType {
            self.next.load(Ordering::Relaxed)
        }
    }
}

#[cfg(feature = "bigint")]
mod mask{
    use num;
    use std::sync::RwLock;

    pub type MaskType = num::BigUint;

    pub struct NextMask{
        next: RwLock<MaskType>,
    }

    impl NextMask {
        pub fn new() -> NextMask {
            NextMask{
                next: RwLock::new(MaskType::from(1u8))
            }
        }

        #[inline]
        pub fn next(&mut self) -> MaskType{
            let mut next = self.next.write().unwrap();
            let ret = next.clone();
            *next *= MaskType::from(2u8);
            ret
        }

        #[inline]
        pub fn get(&self) -> MaskType {
            self.next.read().unwrap().clone()
        }
    }
}

pub use self::mask::{MaskType, NextMask};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Bitmask{
    All,
    Has(MaskType),
    Not(MaskType),
    Or(MaskType),
    HasNot(MaskType, MaskType),
    HasOr(MaskType, MaskType),
    NotOr(MaskType, MaskType),
    HasNotOr(MaskType, MaskType, MaskType),
}

impl Bitmask{
    #[inline]
    pub fn all() -> Bitmask {
        Bitmask::All
    }

    #[inline]
    pub fn has(has: MaskType) -> Bitmask {
        Bitmask::Has(has)
    }

    #[inline]
    pub fn not(not: MaskType) -> Bitmask {
        Bitmask::Not(not)
    }

    #[inline]
    pub fn has_not(has: MaskType, not: MaskType) -> Bitmask {
        Bitmask::HasNot(has, not)
    }

    #[inline]
    pub fn or(or: MaskType) -> Bitmask {
        Bitmask::Or(or)
    }

    #[inline]
    pub fn check(&self, mask: MaskType) -> bool {
        match *self {
            Bitmask::HasNot(ref has, ref not) => mask.clone() & has.clone() == *has && 
                                                 mask.clone() & not.clone() != *not,
            Bitmask::Has(ref has) => mask.clone() & has.clone() == *has,
            Bitmask::Not(ref not) => mask.clone() & not.clone() != *not,
            Bitmask::Or(ref or) => mask.clone() & or.clone() != MaskType::from(0usize),
            Bitmask::HasOr(ref has, ref or) => mask.clone() & has.clone() == *has && 
                                               mask.clone() & or.clone() != MaskType::from(0usize),
            Bitmask::NotOr(ref not, ref or) => mask.clone() & not.clone() != *not && 
                                               mask.clone() & or.clone() != MaskType::from(0usize),
            Bitmask::HasNotOr(ref has, ref not, ref or) => mask.clone() & has.clone() == *has &&  
                                                           mask.clone() & not.clone() != *not && 
                                                           mask.clone() & or.clone() != MaskType::from(0usize),
            Bitmask::All => true,
        }
    }
}

impl BitOr for Bitmask{
    type Output = Bitmask;
    #[inline]
    fn bitor(self, rhs: Bitmask) -> Bitmask {
        use Bitmask::*;
        match (self, rhs){
            (Has(has), Has(rhs))   => Has(has | rhs),
            (Has(has), Not(rhs))   => HasNot(has, rhs),
            (Has(has), HasNot(rhs, rhs_not))      => HasNot(has | rhs, rhs_not),
            (Has(has), Or(rhs))    => HasOr(has, rhs),
            (Has(has), HasOr(rhs, rhs_or))      => HasOr(has | rhs, rhs_or),
            (Has(has), NotOr(rhs_not, rhs_or))    => HasNotOr(has, rhs_not, rhs_or),
            (Has(has), HasNotOr(rhs, rhs_not, rhs_or))    => HasNotOr(has | rhs, rhs_not, rhs_or),
            (Has(has), All)        => Has(has),

            // (Not(not), Has(rhs))   => HasNot(rhs, not),
            (Not(not), Not(rhs))   => Not(not & rhs),
            (Not(not), HasNot(rhs, rhs_not))      => HasNot(rhs, not & rhs_not),
            (Not(not), Or(rhs))    => NotOr(not, rhs),
            (Not(not), HasOr(rhs, rhs_or))      => HasNotOr(rhs, not, rhs_or),
            (Not(not), NotOr(rhs_not, rhs_or))    => NotOr(not & rhs_not, rhs_or),
            (Not(not), HasNotOr(rhs, rhs_not, rhs_or))    => HasNotOr(rhs, not & rhs_not, rhs_or),
            (Not(not), All)        => Not(not),
            
            // (HasNot(has, not), Has(rhs))   => HasNot(has | rhs, not),
            // (HasNot(has, not), Not(rhs))   => HasNot(has, not & rhs),
            (HasNot(has, not), HasNot(rhs, rhs_not))      => HasNot(has | rhs, not & rhs_not),
            (HasNot(has, not), Or(rhs))    => HasNotOr(has, not, rhs),
            (HasNot(has, not), HasOr(rhs, rhs_or))      => HasNotOr(has | rhs, not, rhs_or),
            (HasNot(has, not), NotOr(rhs_not, rhs_or))    => HasNotOr(has, not & rhs_not, rhs_or),
            (HasNot(has, not), HasNotOr(rhs, rhs_not, rhs_or))    => HasNotOr(has | rhs, not | rhs_not, rhs_or),
            (HasNot(has, not), All)        => HasNot(has, not),

            // (Or(or), Has(rhs))   => HasOr(rhs, or),
            // (Or(or), Not(rhs))   => NotOr(rhs, or),
            // (Or(or), HasNot(rhs, rhs_not))      => HasNotOr(rhs, rhs_not, or),
            (Or(or), Or(rhs))    => Or(or | rhs),
            (Or(or), HasOr(rhs, rhs_or))      => HasOr(rhs, or | rhs_or),
            (Or(or), NotOr(rhs_not, rhs_or))    => NotOr(rhs_not, or | rhs_or),
            (Or(or), HasNotOr(rhs, rhs_not, rhs_or))    => HasNotOr(rhs, rhs_not, or | rhs_or),
            (Or(or), All)        => Or(or),

            // (HasOr(has, or), Has(rhs))   => HasOr(has | rhs, or),
            // (HasOr(has, or), Not(rhs))   => HasNotOr(has, rhs, or),
            // (HasOr(has, or), HasNot(rhs, rhs_not))      => HasNotOr(has | rhs, rhs_not, or),
            // (HasOr(has, or), Or(rhs))    => HasOr(has, or | rhs),
            (HasOr(has, or), HasOr(rhs, rhs_or))      => HasOr(has | rhs, or | rhs_or),
            (HasOr(has, or), NotOr(rhs_not, rhs_or))    => HasNotOr(has, rhs_not, or | rhs_or),
            (HasOr(has, or), HasNotOr(rhs, rhs_not, rhs_or))    => HasNotOr(has | rhs, rhs_not, or | rhs_or),
            (HasOr(has, or), All)        => HasOr(has, or),

            // (NotOr(not, or), Has(rhs))   => HasNotOr(rhs, not, or),
            // (NotOr(not, or), Not(rhs))   => NotOr(not & rhs, or),
            // (NotOr(not, or), HasNot(rhs, rhs_not))      => HasNotOr(rhs, not & rhs_not, or),
            // (NotOr(not, or), Or(rhs))    => NotOr(not, or | rhs),
            // (NotOr(not, or), HasOr(rhs, rhs_or))      => HasNotOr(rhs, not, or | rhs_or),
            (NotOr(not, or), NotOr(rhs_not, rhs_or))    => NotOr(not & rhs_not, or | rhs_or),
            (NotOr(not, or), HasNotOr(rhs, rhs_not, rhs_or))    => HasNotOr(rhs, not & rhs_not, or | rhs_or),
            (NotOr(not, or), All)        => NotOr(not, or),
            
            // (HasNotOr(has, not, or), Has(rhs))   => HasNotOr(has | rhs, not, or),
            // (HasNotOr(has, not, or), Not(rhs))   => HasNotOr(has, not & rhs, or),
            // (HasNotOr(has, not, or), HasNot(rhs, rhs_not))      => HasNotOr(has | rhs, not & rhs_not, or),
            // (HasNotOr(has, not, or), Or(rhs))    => HasNotOr(has, not, or | rhs),
            // (HasNotOr(has, not, or), HasOr(rhs, rhs_or))      => HasNotOr(has | rhs, not, or | rhs_or),
            // (HasNotOr(has, not, or), NotOr(rhs_not, rhs_or))    => HasNotOr(has, not & rhs_not, or | rhs_or),
            (HasNotOr(has, not, or), HasNotOr(rhs, rhs_not, rhs_or))    => HasNotOr(has | rhs, not | rhs_not, or | rhs_or),
            (HasNotOr(has, not, or), All)        => HasNotOr(has, not, or),

            // (All, Has(rhs))   => Has(rhs),
            // (All, Not(rhs))   => Not(rhs),
            // (All, HasNot(rhs, rhs_not))    => HasNot(rhs, rhs_not),
            // (All, Or(rhs))    => Or(rhs),
            // (All, HasOr(rhs, rhs_or))    => HasNot(rhs, rhs_or),
            // (All, NotOr(rhs_not, rhs_or))    => HasNot(rhs_not, rhs_or),
            // (All, HasNotOr(rhs, rhs_not, rhs_or))    => HasNotOr(rhs, rhs_not, rhs_or),
            (All, All)        => All,

            (lhs, rhs)        => rhs.bitor(lhs)
        }
    }
}