use num_traits::Bounded;
use serde::de::{self, IntoDeserializer, Visitor};
use serde::ser::Serializer;
use serde::{Deserialize, Deserializer, Serialize};
use std::cmp::Ordering;
use std::cmp::Ordering::*;
use std::fmt;
use std::fmt::{Debug, Display};
use std::marker::PhantomData;
use std::str;
use std::str::FromStr;
use ValueOrLimit::*;

#[derive(Eq, PartialEq, Debug, Copy, Clone)]
pub enum ValueOrLimit<T> {
    Value(T),
    Min,
    Max,
}

impl<T> Bounded for ValueOrLimit<T> {
    fn min_value() -> Self {
        ValueOrLimit::Min
    }
    fn max_value() -> Self {
        ValueOrLimit::Max
    }
}

/**
 * Marker trait for things that serialize as numbers, so we can not accidentally
 * serialize a ValueOrLimit<String>. Serializing ValueOrLimit<String> for a value
 * of "max" or "min" is not reversible.
 */
pub trait SerializesAsNumber {}

impl SerializesAsNumber for u64 {}

impl SerializesAsNumber for i64 {}

impl<T: Serialize + SerializesAsNumber> Serialize for ValueOrLimit<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Max => serializer.serialize_str("max"),
            Min => serializer.serialize_str("min"),
            Value(x) => Serialize::serialize(x, serializer),
        }
    }
}

impl<'de, T: Deserialize<'de> + SerializesAsNumber> Deserialize<'de> for ValueOrLimit<T> {
    fn deserialize<D>(deserializer: D) -> Result<ValueOrLimit<T>, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct MyVisitor<T>(PhantomData<T>);

        impl<'de, T> Visitor<'de> for MyVisitor<T>
        where
            T: Deserialize<'de>,
        {
            type Value = ValueOrLimit<T>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("string or map")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                match v {
                    "min" => Result::Ok(Min),
                    "max" => Result::Ok(Max),
                    v => {
                        let res: Result<T, E> = Deserialize::deserialize(v.into_deserializer());
                        res.map(ValueOrLimit::from)
                    }
                }
            }

            fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                let res: Result<T, E> = Deserialize::deserialize(v.into_deserializer());
                res.map(ValueOrLimit::from)
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                let res: Result<T, E> = Deserialize::deserialize(v.into_deserializer());
                res.map(ValueOrLimit::from)
            }
        }

        deserializer.deserialize_any(MyVisitor(PhantomData))
    }
}

impl<T: Ord> Ord for ValueOrLimit<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Value(a), Value(b)) => a.cmp(&b),
            (Min, Min) => Equal,
            (Max, Max) => Equal,
            (Min, _) => Less,
            (_, Max) => Less,
            (Max, _) => Greater,
            (_, Min) => Greater,
        }
    }
}

impl<T: PartialOrd> PartialOrd for ValueOrLimit<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (Value(a), Value(b)) => a.partial_cmp(&b),
            (Min, Min) => Some(Equal),
            (Max, Max) => Some(Equal),
            (Min, _) => Some(Less),
            (_, Max) => Some(Less),
            (Max, _) => Some(Greater),
            (_, Min) => Some(Greater),
        }
    }
}

impl<T> From<T> for ValueOrLimit<T> {
    fn from(value: T) -> ValueOrLimit<T> {
        Value(value)
    }
}

impl<T: Display> Display for ValueOrLimit<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value(x) => write!(f, "Value({})", x),
            Min => write!(f, "min"),
            Max => write!(f, "max"),
        }
    }
}

#[derive(Debug)]
pub enum ValueOrLimitError<E: Debug + Display> {
    FormatError,
    Nested(E),
}

impl<E: Display + Debug> Display for ValueOrLimitError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValueOrLimitError::FormatError => write!(f, "FormatError: value must be enclosed in Value(...)"),
            ValueOrLimitError::Nested(e) => write!(f, "Error parsing value: {}", e),
        }
    }
}

impl<T: FromStr> FromStr for ValueOrLimit<T>
where
    <T as FromStr>::Err: Debug + Display,
{
    type Err = ValueOrLimitError<<T as FromStr>::Err>;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "min" => Result::Ok(Min),
            "max" => Result::Ok(Max),
            text => {
                let end = text.len();
                if end < 7 || &text[0..6] != "Value(" || &text[end - 1..end] != ")" {
                    Result::Err(ValueOrLimitError::FormatError)
                } else {
                    let res = FromStr::from_str(&text[6..end - 1]);
                    res.map(Value).map_err(ValueOrLimitError::Nested)
                }
            }
        }
    }
}

impl<T> ValueOrLimit<T> {
    pub fn into_value(self, min: T, max: T) -> T {
        match self {
            Value(x) => x,
            Min => min,
            Max => max,
        }
    }

    /// *Careful:* This applies the given transformation to Values only,
    /// leaving Min and Max alone.
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> ValueOrLimit<U> {
        match self {
            Value(x) => Value(f(x)),
            Min => Min,
            Max => Max,
        }
    }

    pub fn and_then<U>(self, f: impl FnOnce(T) -> ValueOrLimit<U>) -> ValueOrLimit<U> {
        match self {
            Value(x) => f(x),
            Min => Min,
            Max => Max,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compare() {
        // FIXME test equality as well
        assert!(ValueOrLimit::<u64>::Max > ValueOrLimit::Min);
        assert!(ValueOrLimit::<u64>::Min < ValueOrLimit::Max);

        assert!(ValueOrLimit::Min < ValueOrLimit::from(0));
        assert!(ValueOrLimit::Min < ValueOrLimit::from(42));
        assert!(ValueOrLimit::Min < ValueOrLimit::from(u64::max_value()));
        assert!(ValueOrLimit::from(0) > ValueOrLimit::Min);
        assert!(ValueOrLimit::from(42) > ValueOrLimit::Min);
        assert!(ValueOrLimit::from(u64::max_value()) > ValueOrLimit::Min);

        assert!(ValueOrLimit::from(0) < ValueOrLimit::from(1));

        assert!(ValueOrLimit::from(0) < ValueOrLimit::Max);
        assert!(ValueOrLimit::from(42) < ValueOrLimit::Max);
        assert!(ValueOrLimit::from(u64::max_value()) < ValueOrLimit::Max);
        assert!(ValueOrLimit::Max > ValueOrLimit::from(0));
        assert!(ValueOrLimit::Max > ValueOrLimit::from(42));
        assert!(ValueOrLimit::Max > ValueOrLimit::from(u64::max_value()));
    }

    #[test]
    fn test_from_str() {
        type T = ValueOrLimit<u8>;

        // cannot use T here, feature not yet stable
        let min = ValueOrLimit::<u8>::Min;
        let med = ValueOrLimit::from(63_u8);
        let max = ValueOrLimit::<u8>::Max;

        let f_min = &format!("{}", min)[..];
        let f_med = &format!("{}", med)[..];
        let f_max = &format!("{}", max)[..];
        assert_eq!(f_min, "min");
        assert_eq!(f_med, "Value(63)");
        assert_eq!(f_max, "max");
        assert_eq!(f_min.parse::<ValueOrLimit<u8>>().unwrap(), min);
        assert_eq!(f_med.parse::<ValueOrLimit<u8>>().unwrap(), med);
        assert_eq!(f_max.parse::<ValueOrLimit<u8>>().unwrap(), max);

        let unknown = "bla".parse::<T>().expect_err("unknown should err");
        assert_eq!(format!("{:?}", unknown), "FormatError");

        let alpha = "Value(x)".parse::<T>().expect_err("alpha should err");
        assert_eq!(format!("{:?}", alpha), "Nested(ParseIntError { kind: InvalidDigit })");

        let toobig = "Value(300)".parse::<T>().expect_err("toobig should err");
        assert_eq!(format!("{:?}", toobig), "Nested(ParseIntError { kind: PosOverflow })");
    }

    #[test]
    fn test_into_value() {
        assert!(ValueOrLimit::<u64>::Min.into_value(0, 1000) == 0);
        assert!(ValueOrLimit::<u64>::Max.into_value(0, 1000) == 1000);
        assert!(ValueOrLimit::<u64>::from(3).into_value(0, 1000) == 3);
    }
}
