use core::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign};

pub struct Vec2D {
    pub x: f32,
    pub y: f32,
}

impl Vec2D {
    /// Vector with x = 1, y = 1
    pub const ZERO: Self = Self::new(0f32, 0f32);
    pub const ONE: Self = Self::new(1f32, 1f32);
    pub const UNIT_X: Self = Self::new(1f32, 0f32);
    pub const UNIT_Y: Self = Self::new(0f32, 1f32);

    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn length_sq(&self) -> f32 {
        self.x * self.x + self.y * self.y
    }
    pub fn manhattan_dist(&self, rhs: Self) -> f32 {
        (rhs.x - self.x).abs() + (rhs.y - self.y).abs()
    }
}

impl Mul for Vec2D {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self::new(self.x * rhs.x, self.y * rhs.y)
    }
}

impl Add for Vec2D {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.x + rhs.x, self.y * rhs.y)
    }
}

impl Sub for Vec2D {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl Div for Vec2D {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        Self::new(self.x / rhs.x, self.y / rhs.y)
    }
}

impl AddAssign for Vec2D {
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl DivAssign for Vec2D {
    fn div_assign(&mut self, rhs: Self) {
        self.x /= rhs.x;
        self.y /= rhs.y
    }
}
impl DivAssign<f32> for Vec2D {
    fn div_assign(&mut self, rhs: f32) {
        self.x /= rhs;
        self.y /= rhs;
    }
}

impl MulAssign for Vec2D {
    fn mul_assign(&mut self, rhs: Self) {
        self.x *= rhs.x;
        self.y *= rhs.y
    }
}

impl MulAssign<f32> for Vec2D {
    fn mul_assign(&mut self, rhs: f32) {
        self.x *= rhs;
        self.y *= rhs;
    }
}

impl SubAssign for Vec2D {
    fn sub_assign(&mut self, rhs: Self) {
        self.x -= rhs.x;
        self.y -= rhs.y;
    }
}
