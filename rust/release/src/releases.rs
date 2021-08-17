use anyhow::{anyhow, Error};
use std::cmp::Ordering;
use std::cmp::Reverse;
use std::fmt;
use std::fmt::Display;
use std::str::FromStr;

use crate::products::Product;
use crate::versions::Version;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Release {
    pub product: Product,
    pub version: Version,
}
impl PartialOrd for Release {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Release {
    fn cmp(&self, other: &Self) -> Ordering {
        self.product
            .cmp(&other.product)
            .then_with(|| Reverse(&self.version).cmp(&Reverse(&other.version)))
    }
}

impl Display for Release {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}-{}", self.product, self.version)
    }
}

impl FromStr for Release {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.contains(char::is_whitespace) {
            return Err(anyhow!("release '{}' contains whitespace", s));
        }
        let parts: Vec<&str> = s.split('-').collect();
        if parts.len() < 2 {
            return Err(anyhow!("unable to parse release tag {}", s));
        }

        let (ver_str, item_parts) = parts.split_last().unwrap();

        let product = Product::from_str(item_parts.join("-").as_str())?;
        let version = Version::from_str(ver_str)?;
        Ok(Release { product, version })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    impl Release {
        pub fn new(product: Product, major: u64, minor: u64, patch: u64) -> Release {
            Release {
                product,
                version: Version::new(major, minor, patch),
            }
        }
    }
    #[test]
    fn test_release_new() {
        assert_eq!(
            Release::new(Product::Actyx, 1, 2, 3),
            Release {
                product: Product::Actyx,
                version: Version::new(1, 2, 3)
            }
        );
    }
    #[test]
    fn test_release_fmt() {
        assert_eq!(format!("{}", Release::new(Product::Actyx, 1, 2, 3)), "actyx-1.2.3");
    }
    #[test]
    fn release_from_str() -> Result<(), Error> {
        assert_eq!(Release::from_str("actyx-1.2.3")?, Release::new(Product::Actyx, 1, 2, 3));
        assert_eq!(
            Release::from_str("node-manager-1.2.3")?,
            Release::new(Product::NodeManager, 1, 2, 3)
        );
        assert!(Release::from_str("item-1.2").is_err());
        assert!(Release::from_str("item-1.2.3.4").is_err());
        assert!(Release::from_str("abc item-1.3.4").is_err());
        Ok(())
    }
}
