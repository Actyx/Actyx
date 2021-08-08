use crate::{products::Product, versions::VersionImpact};
use anyhow::{anyhow, Error};
use regex::Regex;
use std::fmt::Display;
use std::str::FromStr;

#[derive(Debug, Copy, Clone, Eq, PartialEq, PartialOrd, Ord)]
pub struct ChangeKind {
    pub ord: u32,
    pub displ: &'static str,
    pub from: &'static str,
    pub impact: VersionImpact,
}

impl ChangeKind {
    pub const BREAK: ChangeKind = ChangeKind {
        ord: 9,
        displ: "Breaking change",
        from: "break",
        impact: VersionImpact::BumpMajor,
    };
    pub const FEAT: ChangeKind = ChangeKind {
        ord: 8,
        displ: "New feature",
        from: "feat",
        impact: VersionImpact::BumpMinor,
    };
    pub const PERF: ChangeKind = ChangeKind {
        ord: 7,
        displ: "Performance",
        from: "perf",
        impact: VersionImpact::BumpPatch,
    };
    pub const FIX: ChangeKind = ChangeKind {
        ord: 6,
        displ: "Bug fix",
        from: "fix",
        impact: VersionImpact::BumpPatch,
    };
    pub const STYLE: ChangeKind = ChangeKind {
        ord: 5,
        displ: "Style",
        from: "style",
        impact: VersionImpact::BumpPatch,
    };
    pub const DOCS: ChangeKind = ChangeKind {
        ord: 4,
        displ: "Docs",
        from: "docs",
        impact: VersionImpact::BumpPatch,
    };
    pub const TEST: ChangeKind = ChangeKind {
        ord: 3,
        displ: "Test",
        from: "test",
        impact: VersionImpact::BumpPatch,
    };
    pub const REFACTOR: ChangeKind = ChangeKind {
        ord: 2,
        displ: "Refactor",
        from: "refactor",
        impact: VersionImpact::BumpPatch,
    };
    pub const BUILD: ChangeKind = ChangeKind {
        ord: 1,
        displ: "Build",
        from: "build",
        impact: VersionImpact::BumpPatch,
    };
    pub const CI: ChangeKind = ChangeKind {
        ord: 0,
        displ: "CI",
        from: "ci",
        impact: VersionImpact::BumpPatch,
    };
    pub const ALL: &'static [ChangeKind] = &[
        Self::BREAK,
        Self::FEAT,
        Self::PERF,
        Self::FIX,
        Self::STYLE,
        Self::DOCS,
        Self::TEST,
        Self::REFACTOR,
        Self::BUILD,
        Self::CI,
    ];
}

impl Display for ChangeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.displ)
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct Change {
    pub kind: ChangeKind,
    pub product: Product,
    pub message: String,
}

impl Change {
    pub fn new(kind: ChangeKind, product: Product, message: String) -> Change {
        Change { kind, product, message }
    }
}

fn change_from_str(s: &str, product: Product, message: String) -> Result<Change, Error> {
    ChangeKind::ALL
        .iter()
        .find(|k| k.from == s)
        .map(|k| Change::new(*k, product, message))
        .ok_or_else(|| anyhow!("unknown scope {}", s))
}

pub fn try_change_from_line(s: &str) -> Option<Change> {
    Change::from_str(s).ok()
}

impl Display for Change {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}({}): {}", self.kind.displ, self.product, self.message)
    }
}

impl FromStr for Change {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Try feat
        let re = Regex::new(r"^(\w+)\(([-\w]+)\): ([^\n]+)\n?$").unwrap();
        let caps = re.captures(s);
        match caps {
            None => Err(anyhow!("unable to parse {}", s)),
            Some(caps_) => {
                let kind = caps_.get(1).unwrap().as_str();
                let scope = Product::from_str(caps_.get(2).unwrap().as_str());
                let message = caps_.get(3).unwrap().as_str();
                match scope {
                    Err(e) => Err(e),
                    Ok(s) => change_from_str(kind, s, message.to_string()),
                }
            }
        }
    }
}

#[test]
fn test_all_change_kinds_correctly_listed() {
    // This is so force you to count the kinds and
    // adjust `ChangeKind::ALL` if you add a kind.
    assert_eq!(ChangeKind::ALL.len(), 10)
}
#[test]
#[allow(clippy::eq_op)]
fn test_kind_ordering() {
    assert!(ChangeKind::BREAK > ChangeKind::FEAT);
    assert!(ChangeKind::FEAT < ChangeKind::BREAK);
    assert!(ChangeKind::FEAT != ChangeKind::BREAK);
    assert!(ChangeKind::BREAK == ChangeKind::BREAK);
    assert!(ChangeKind::FEAT == ChangeKind::FEAT);
    assert!(ChangeKind::DOCS < ChangeKind::FEAT);
    assert!(ChangeKind::FIX > ChangeKind::DOCS);
    assert!(ChangeKind::DOCS == ChangeKind::DOCS);
}
#[test]
fn test_all_kinds_ordered_correctly() {
    for i in 1..ChangeKind::ALL.len() {
        assert!(
            ChangeKind::ALL[i - 1] > ChangeKind::ALL[i],
            "expected element {} ({}) of ALL to be greater than element {} ({})",
            i - 1,
            ChangeKind::ALL[i - 1],
            i,
            ChangeKind::ALL[i]
        );
    }
}

#[test]
fn test_docs_from_str() {
    assert!(Change::from_str("doc(actyx): message").is_err());
    assert!(Change::from_str("docs(unknown): message").is_err());
    assert!(Change::from_str("docs(unknown):").is_err());
    assert!(Change::from_str("(unknown): asd").is_err());
    assert!(Change::from_str("docs(actyx): asd\n").is_ok());
}
#[test]
fn test_from_iterated() -> Result<(), Error> {
    for kind in ChangeKind::ALL.iter() {
        for product in vec![
            Product::Actyx,
            Product::Cli,
            Product::NodeManager,
            Product::Pond,
            Product::TsSdk,
            Product::RustSdk,
            Product::CSharpSdk,
        ] {
            assert_eq!(
                Change::from_str(format!("{}({}): m", kind.from, product).as_str())?,
                Change::new(*kind, product, "m".to_string())
            )
        }
    }
    Ok(())
}

#[test]
fn test_docs_from_str_scopes() -> Result<(), Error> {
    assert_eq!(
        Change::from_str("docs(actyx): message")?,
        Change::new(ChangeKind::DOCS, Product::Actyx, "message".to_string())
    );
    assert_eq!(
        Change::from_str("docs(cli): message")?,
        Change::new(ChangeKind::DOCS, Product::Cli, "message".to_string())
    );
    assert_eq!(
        Change::from_str("docs(node-manager): message")?,
        Change::new(ChangeKind::DOCS, Product::NodeManager, "message".to_string())
    );
    assert_eq!(
        Change::from_str("docs(pond): message")?,
        Change::new(ChangeKind::DOCS, Product::Pond, "message".to_string())
    );
    assert_eq!(
        Change::from_str("docs(ts-sdk): message")?,
        Change::new(ChangeKind::DOCS, Product::TsSdk, "message".to_string())
    );
    assert_eq!(
        Change::from_str("docs(rust-sdk): message")?,
        Change::new(ChangeKind::DOCS, Product::RustSdk, "message".to_string())
    );
    Ok(())
}

#[test]
fn test_docs_from_str_kinds() -> Result<(), Error> {
    assert_eq!(
        Change::from_str("docs(actyx): message")?,
        Change::new(ChangeKind::DOCS, Product::Actyx, "message".to_string())
    );
    assert_eq!(
        Change::from_str("feat(actyx): message")?,
        Change::new(ChangeKind::FEAT, Product::Actyx, "message".to_string())
    );
    assert_eq!(
        Change::from_str("fix(actyx): message")?,
        Change::new(ChangeKind::FIX, Product::Actyx, "message".to_string())
    );
    assert_eq!(
        Change::from_str("perf(actyx): message")?,
        Change::new(ChangeKind::PERF, Product::Actyx, "message".to_string())
    );
    assert_eq!(
        Change::from_str("refactor(actyx): message")?,
        Change::new(ChangeKind::REFACTOR, Product::Actyx, "message".to_string())
    );
    assert_eq!(
        Change::from_str("style(actyx): message")?,
        Change::new(ChangeKind::STYLE, Product::Actyx, "message".to_string())
    );
    assert_eq!(
        Change::from_str("test(actyx): message")?,
        Change::new(ChangeKind::TEST, Product::Actyx, "message".to_string())
    );
    assert_eq!(
        Change::from_str("ci(actyx): message")?,
        Change::new(ChangeKind::CI, Product::Actyx, "message".to_string())
    );
    assert_eq!(
        Change::from_str("build(actyx): message")?,
        Change::new(ChangeKind::BUILD, Product::Actyx, "message".to_string())
    );
    assert_eq!(
        Change::from_str("break(actyx): message")?,
        Change::new(ChangeKind::BREAK, Product::Actyx, "message".to_string())
    );
    Ok(())
}
