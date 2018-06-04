use std::ops::{Add, Mul};
use std::fmt;

#[derive(Copy, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Uf64 {
    value: f64,
    rstd: f64,
}

impl Uf64 {
    #[allow(dead_code)]
    pub fn std(&self) -> f64 {
        self.var().sqrt()
    }

    pub fn rstd(&self) -> f64 {
        // self.std() / self.value
        self.rstd
    }

    pub fn rvar(&self) -> f64 {
        // self.std() / self.value
        self.rstd().powi(2)
    }

    pub fn var(&self) -> f64 {
        (self.value() * self.rstd()).powi(2)
    }

    pub fn value(&self) -> f64 {
        self.value
    }

    #[allow(dead_code)]
    pub fn from_value(value: f64) -> Self {
        Self::from_value_var(value, 0.)
    }

    pub fn from_value_rstd(value: f64, rstd: f64) -> Self {
        Self { value, rstd }
    }

    #[allow(dead_code)]
    pub fn from_value_std(value: f64, std: f64) -> Self {
        let var = std * std;
        Self::from_value_var(value, var)
    }

    #[allow(dead_code)]
    pub fn from_value_var(value: f64, var: f64) -> Self {
        let rstd = var.sqrt() / value;
        Self { value, rstd }
    }
}

impl Add for Uf64 {
    type Output = Uf64;

    fn add(self: Uf64, other: Uf64) -> Uf64 {
        let val1 = self.value();
        let val2 = other.value();
        let var1 = self.var();
        let var2 = other.var();
        Uf64::from_value_var(val1 + val2, var1 + var2)
    }
}

impl Mul for Uf64 {
    type Output = Uf64;

    fn mul(self: Uf64, other: Uf64) -> Uf64 {
        let value = self.value() * other.value();
        let rstd = (self.rvar() + other.rvar()).sqrt();
        Uf64::from_value_rstd(value, rstd)
    }
}

impl fmt::Display for Uf64 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let value = self.value();
        let pstd = self.rstd() * 100.;
        write!(f, "{} +- {}%", value, pstd)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(non_snake_case)]
    fn test_Uf64() {
        let u1 = Uf64::from_value_var(1., 100.);
        assert_eq!(u1.value(), 1.);
        assert_eq!(u1.var(), 100.);
        assert_eq!(u1.std(), 10.);
        assert_eq!(u1.rstd(), 10.);
        let u2 = u1 + u1;
        assert_relative_eq!(u2.value(), 2.);
        assert_relative_eq!(u2.var(), 200.);
        let u3 = Uf64::from_value_rstd(10., 0.1);
        assert_relative_eq!(u3.var(), 1.);

        let c1 = Uf64::from_value_var(23., 0.);
        assert_relative_eq!(u1.var(), (u1 + c1).var());
        assert_relative_eq!(u1.std() * c1.value(), (u1 * c1).std());
    }

    quickcheck! {
        fn prop_inclusion_multipicative(x:f64, y:f64) -> bool {
            Uf64::from_value(x) * Uf64::from_value(y)
                == Uf64::from_value(x*y)
        }

        fn prop_inclusion_additive(x:f64, y:f64) -> bool {
            Uf64::from_value(x) + Uf64::from_value(y)
                == Uf64::from_value(x+y)
        }
    }
}
