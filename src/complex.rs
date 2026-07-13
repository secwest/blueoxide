/// A compact complex sample type used to avoid a runtime dependency for basic
/// I/Q operations.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct Complex32 {
    pub re: f32,
    pub im: f32,
}

impl Complex32 {
    pub const ZERO: Self = Self::new(0.0, 0.0);

    pub const fn new(re: f32, im: f32) -> Self {
        Self { re, im }
    }

    pub fn magnitude_squared(self) -> f32 {
        self.re.mul_add(self.re, self.im * self.im)
    }

    /// Returns the phase of `current * conjugate(self)`.
    pub fn phase_difference(self, current: Self) -> f32 {
        let dot = self.re.mul_add(current.re, self.im * current.im);
        let cross = self.re.mul_add(current.im, -self.im * current.re);
        cross.atan2(dot)
    }
}
