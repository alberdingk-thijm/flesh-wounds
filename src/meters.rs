//! Helper module for tracking variables with a current state out of some original state.

use std::fmt;
use std::ops::{Add, AddAssign, Sub, SubAssign};
use std::str::FromStr;
use std::num::ParseIntError;

/// Struct for tracking the total of .0 out of .1
#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq)]
pub struct Meter<T: Copy + Clone>(T, T);

impl<T: Copy + Clone> Meter<T> {
    pub fn curr(&self) -> T {
        self.0
    }

    pub fn max(&self) -> T {
        self.1
    }
}

impl<T : Copy + Clone + FromStr<Err = ParseIntError>> FromStr for Meter<T> {
    type Err = ParseIntError;
    /// Parse a string depicting a fraction as a Meter.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let terms : Vec<&str> = s.split("/").collect();
        let curr_t = terms[0].parse::<T>()?;
        let max_t = terms[1].parse::<T>()?;
        Ok(Meter(curr_t, max_t))
    }
}

impl<T: Copy + Clone + Ord + Add<Output = T>> Add<T> for Meter<T> {
    type Output = Meter<T>;
    fn add(self, rhs: T) -> Self::Output {
        Meter(self.1.min(self.0 + rhs), self.1)
    }
}

impl<T: Copy + Clone + Ord + Add<Output = T>> AddAssign<T> for Meter<T> {
    fn add_assign(&mut self, rhs: T) {
        self.0 = self.1.min(self.0 + rhs);
    }
}

impl<T: Copy + Clone + Ord + Sub<Output = T>> Sub<T> for Meter<T> {
    type Output = Meter<T>;
    fn sub(self, rhs: T) -> Self::Output {
        Meter(self.0 - rhs, self.1)
    }
}

impl<T: Copy + Clone + Ord + SubAssign> SubAssign<T> for Meter<T> {
    fn sub_assign(&mut self, rhs: T) {
        self.0 -= rhs;
    }
}

impl<T: Copy + Clone + fmt::Display> fmt::Display for Meter<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}/{}", self.0, self.1)
    }
}

/// Struct for tracking the total amount of .0, which increases each turn by .1
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Incrementer(f64, f64);

impl Incrementer {
    pub fn new(incr: f64) -> Self {
        Incrementer(0.0, incr)
    }

    pub fn curr(&self) -> f64 {
        self.0
    }

    pub fn incr(&mut self) {
        self.0 += self.1;
    }

    pub fn decr(&mut self, x: f64) {
        // prevent from decrementing into negatives
        self.0 -= if self.0 < x { self.0 } else { x };
    }
}

impl fmt::Display for Incrementer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:.2}/{:.2}", self.0, self.1)
    }
}
