extern crate derive_more;
use anyhow::{anyhow, Error};
use derive_more::{Display, From};
use std::str::FromStr;

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Display, From)]
pub enum Product {
    #[display(fmt = "actyx")]
    Actyx,
    #[display(fmt = "cli")]
    Cli,
    #[display(fmt = "node-manager")]
    NodeManager,
    #[display(fmt = "pond")]
    Pond,
    #[display(fmt = "ts-sdk")]
    TsSdk,
    #[display(fmt = "rust-sdk")]
    RustSdk,
    #[display(fmt = "csharp-sdk")]
    CSharpSdk,
}

impl Product {
    pub const ALL: [Product; 7] = [
        Self::Actyx,
        Self::Cli,
        Self::NodeManager,
        Self::Pond,
        Self::TsSdk,
        Self::RustSdk,
        Self::CSharpSdk,
    ];
}

impl FromStr for Product {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "actyx" => Ok(Product::Actyx),
            "cli" => Ok(Product::Cli),
            "node-manager" => Ok(Product::NodeManager),
            "pond" => Ok(Product::Pond),
            "ts-sdk" => Ok(Product::TsSdk),
            "rust-sdk" => Ok(Product::RustSdk),
            "csharp-sdk" => Ok(Product::CSharpSdk),
            _ => Err(anyhow!("unknown product {}", s)),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn add_to_enum() {
        assert_eq!(Product::ALL.len(), 7);
    }

    #[test]
    fn test_from_str() -> Result<(), Error> {
        use Product::*;
        assert_eq!(Product::from_str("actyx")?, Actyx);
        assert_eq!(Product::from_str("cli")?, Cli);
        assert_eq!(Product::from_str("node-manager")?, NodeManager);
        assert_eq!(Product::from_str("pond")?, Pond);
        assert_eq!(Product::from_str("ts-sdk")?, TsSdk);
        assert_eq!(Product::from_str("rust-sdk")?, RustSdk);
        assert_eq!(Product::from_str("csharp-sdk")?, CSharpSdk);
        Ok(())
    }

    #[test]
    fn test_fmt() {
        use Product::*;
        assert_eq!(format!("{}", Actyx), "actyx");
        assert_eq!(format!("{}", Cli), "cli");
        assert_eq!(format!("{}", NodeManager), "node-manager");
        assert_eq!(format!("{}", Pond), "pond");
        assert_eq!(format!("{}", TsSdk), "ts-sdk");
        assert_eq!(format!("{}", RustSdk), "rust-sdk");
        assert_eq!(format!("{}", CSharpSdk), "csharp-sdk");
    }
}
