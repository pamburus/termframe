use num_integer::Integer;
use num_traits::CheckedMul;

pub trait SnapUp: Sized {
    fn snap_up(self, step: Self) -> Self;
    fn checked_snap_up(self, step: Self) -> Option<Self>;
}

// ---- seal the blanket impl so it applies ONLY to integer primitives ----
mod sealed {
    pub trait Sealed {}
    impl Sealed for i8 {}
    impl Sealed for i16 {}
    impl Sealed for i32 {}
    impl Sealed for i64 {}
    impl Sealed for i128 {}
    impl Sealed for isize {}
    impl Sealed for u8 {}
    impl Sealed for u16 {}
    impl Sealed for u32 {}
    impl Sealed for u64 {}
    impl Sealed for u128 {}
    impl Sealed for usize {}
}

/* -------- Integers: blanket impl, but sealed -------- */
impl<T> SnapUp for T
where
    T: sealed::Sealed + Integer + CheckedMul + Copy,
{
    fn snap_up(self, step: Self) -> Self {
        assert!(step != T::zero(), "step must be non-zero");
        // mathematically correct ceil division (handles negatives)
        self.div_ceil(&step) * step
    }

    fn checked_snap_up(self, step: Self) -> Option<Self> {
        if step == T::zero() {
            return None;
        }
        let q = self.div_ceil(&step);
        q.checked_mul(&step)
    }
}

/* -------- Floats: concrete impls -------- */
impl SnapUp for f32 {
    fn snap_up(self, step: Self) -> Self {
        assert!(step > 0.0, "step must be > 0");
        (self / step).ceil() * step
    }
    fn checked_snap_up(self, step: Self) -> Option<Self> {
        if step <= 0.0 {
            return None;
        }
        Some((self / step).ceil() * step)
    }
}

impl SnapUp for f64 {
    fn snap_up(self, step: Self) -> Self {
        assert!(step > 0.0, "step must be > 0");
        (self / step).ceil() * step
    }
    fn checked_snap_up(self, step: Self) -> Option<Self> {
        if step <= 0.0 {
            return None;
        }
        Some((self / step).ceil() * step)
    }
}
