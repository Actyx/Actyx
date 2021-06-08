pub use semver::Version;

use crate::changes::Change;
use crate::products::Product;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub enum VersionImpact {
    None,
    BumpPatch,
    BumpMinor,
    BumpMajor,
}

/// Returns none if the array if empty
pub fn get_highest_impact(impacts: &[VersionImpact]) -> VersionImpact {
    impacts.iter().max().copied().unwrap_or(VersionImpact::None)
}

pub fn from_change_and_product(target_product: &Product, change: &Change) -> VersionImpact {
    if target_product != &change.product {
        VersionImpact::None
    } else {
        change.kind.impact
    }
}

pub fn apply_impact(current: &Version, impact: &VersionImpact) -> Version {
    let mut new = current.clone();
    match impact {
        VersionImpact::None => (),
        VersionImpact::BumpPatch => new.increment_patch(),
        VersionImpact::BumpMinor => new.increment_minor(),
        VersionImpact::BumpMajor => new.increment_major(),
    };
    new
}

pub fn apply_changes(product: &Product, current_version: &Version, changes: &[Change]) -> Version {
    apply_impact(
        current_version,
        &get_highest_impact(
            &*changes
                .iter()
                .map(|c| from_change_and_product(product, c))
                .collect::<Vec<VersionImpact>>(),
        ),
    )
}

#[cfg(test)]
mod test {

    use crate::changes::ChangeKind;

    use super::*;

    #[test]
    fn from_change_and_product_test() {
        assert_eq!(
            from_change_and_product(
                &Product::Actyx,
                &Change::new(ChangeKind::BREAK, Product::Cli, "".to_string())
            ),
            VersionImpact::None
        );
        assert_eq!(
            from_change_and_product(
                &Product::Actyx,
                &Change::new(ChangeKind::BREAK, Product::Actyx, "".to_string())
            ),
            VersionImpact::BumpMajor
        );
        assert_eq!(
            from_change_and_product(
                &Product::Actyx,
                &Change::new(ChangeKind::FEAT, Product::Actyx, "".to_string())
            ),
            VersionImpact::BumpMinor
        );
        assert_eq!(
            from_change_and_product(
                &Product::Actyx,
                &Change::new(ChangeKind::PERF, Product::Actyx, "".to_string())
            ),
            VersionImpact::BumpPatch
        );
        assert_eq!(
            from_change_and_product(
                &Product::Actyx,
                &Change::new(ChangeKind::CI, Product::Actyx, "".to_string())
            ),
            VersionImpact::BumpPatch
        );
    }

    #[test]
    fn apply_impact_test() {
        let base = Version::new(1, 2, 3);
        assert_eq!(apply_impact(&base, &VersionImpact::None), base);
        assert_eq!(
            apply_impact(&base, &VersionImpact::BumpPatch),
            Version::new(1, 2, 4)
        );
        assert_eq!(
            apply_impact(&base, &VersionImpact::BumpMinor),
            Version::new(1, 3, 0)
        );
        assert_eq!(
            apply_impact(&base, &VersionImpact::BumpMajor),
            Version::new(2, 0, 0)
        );
    }

    #[test]
    fn impact_ord_test() {
        assert!(VersionImpact::None < VersionImpact::BumpPatch);
        assert!(VersionImpact::BumpPatch < VersionImpact::BumpMinor);
        assert!(VersionImpact::BumpMinor < VersionImpact::BumpMajor);
        assert!(VersionImpact::BumpMajor > VersionImpact::BumpPatch);
    }

    #[test]
    fn impact_vec_sort() {
        let mut sorted = vec![
            VersionImpact::None,
            VersionImpact::BumpMinor,
            VersionImpact::BumpMajor,
            VersionImpact::BumpPatch,
        ];
        sorted.sort();
        assert_eq!(&VersionImpact::BumpMajor, sorted.last().unwrap());
    }
}
