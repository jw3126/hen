use errors::*;
use std::fmt;
use std::convert::From;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Omittable<T> {
    Omitted,
    Fail(String),
    Available(T),
}

impl<T> From<Result<T>> for Omittable<T> {
    fn from(res: Result<T>) -> Self {
        Omittable::from(res.into_stub())
    }
}

impl<T> From<StubResult<T>> for Omittable<T> {
    fn from(res: StubResult<T>) -> Self {
        match res {
            Ok(value) => Omittable::Available(value),
            Err(s) => Omittable::Fail(s),
        }
    }
}

impl<T> Omittable<T> {
    pub fn is_available(&self) -> bool {
        match self {
            &Omittable::Available(_) => true,
            _ => false,
        }
    }

    pub fn into_stub_result(self) -> StubResult<T> {
        match self {
            Omittable::Fail(s) => Err(s),
            Omittable::Omitted => Err("Omitted".to_string()),
            Omittable::Available(t) => Ok(t),
        }
    }

    #[allow(dead_code)]
    pub fn map<U, F: Fn(T) -> U>(self, f: F) -> Omittable<U> {
        match self {
            Omittable::Available(value) => Omittable::Available(f(value)),
            Omittable::Fail(s) => Omittable::Fail(s.clone()),
            Omittable::Omitted => Omittable::Omitted,
        }
    }

    pub fn map2<S, U, F: Fn(S, T) -> U>(f: F, s: Omittable<S>, t: Omittable<T>) -> Omittable<U> {
        match s {
            Omittable::Fail(msg) => Omittable::Fail(msg),

            Omittable::Omitted => match t {
                Omittable::Fail(msg) => Omittable::Fail(msg),
                _ => Omittable::Omitted,
            },

            Omittable::Available(s_val) => match t {
                Omittable::Available(t_val) => Omittable::Available(f(s_val, t_val)),
                Omittable::Omitted => Omittable::Omitted,
                Omittable::Fail(msg) => Omittable::Fail(msg),
            },
        }
    }
}

impl<T> fmt::Display for Omittable<T>
where
    T: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Omittable::Omitted => writeln!(f, ""),
            &Omittable::Fail(ref msg) => writeln!(f, "{}", msg),
            &Omittable::Available(ref x) => writeln!(f, "{}", x),
        }
    }
}
