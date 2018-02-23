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
    HasNot(MaskType, MaskType)
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
    pub fn check(&self, mask: MaskType) -> bool {
        match *self {
            Bitmask::HasNot(ref has, ref not) => mask.clone() & has.clone() == *has && mask.clone() & not.clone() != *not,
            Bitmask::Has(ref has) => mask.clone() & has.clone() == *has,
            Bitmask::Not(ref not) => mask.clone() & not.clone() != *not,
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
            (Has(has), All)        => Has(has),

            (Not(not), Has(rhs))   => HasNot(rhs, not),
            (Not(not), Not(rhs))   => Not(not & rhs),
            (Not(not), HasNot(rhs, rhs_not))      => HasNot(rhs, not & rhs_not),
            (Not(not), All)        => Not(not),
            
            (HasNot(has, not), Has(rhs))   => HasNot(has | rhs, not),
            (HasNot(has, not), Not(rhs))   => HasNot(has, not & rhs),
            (HasNot(has, not), HasNot(rhs, rhs_not))      => HasNot(has | rhs, not & rhs_not),
            (HasNot(has, not), All)        => HasNot(has, not),

            (All, Has(rhs))   => Has(rhs),
            (All, Not(rhs))   => Not(rhs),
            (All, HasNot(rhs, rhs_not))    => HasNot(rhs, rhs_not),
            (All, All)        => All,
        }
    }
}