use actyx_sdk::AppId;
use derive_more::{Display, Error};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{iter::repeat, str::FromStr};

#[derive(Debug, Display, Error)]
#[display(fmt = "Required form '<tld>.<apex>.*'. Received: {}.", input)]
pub struct InvalidAppDomainForm {
    input: String,
}

// MVP required form  <tld>.<apex>.*, which allows any subdomain as well. com.example.*.info is currently not supported.
#[derive(Debug, Display, Clone, Deserialize, Serialize, PartialEq)]
pub struct AppDomain(String);

impl AppDomain {
    pub fn is_app_id_allowed(&self, app_id: &AppId) -> bool {
        self.0
            .split('.')
            .zip(app_id.split('.').chain(repeat("")))
            .all(|(me, them)| (me == "*" && !them.is_empty()) || me == them)
    }
}

impl FromStr for AppDomain {
    type Err = InvalidAppDomainForm;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let re = Regex::new("^[a-z0-9-]+\\.[a-z0-9-]+\\.\\*$").unwrap();
        match re.is_match(&s) {
            true => Ok(Self(s.into())),
            false => Err(InvalidAppDomainForm { input: s.into() }),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use actyx_sdk::AppId;

    use crate::app_domain::InvalidAppDomainForm;

    use super::AppDomain;

    #[test]
    fn should_fail() {
        vec![
            "com.example.*.info",
            "com.actyx.test...",
            "",
            "actyx",
            "com.actyx",
            "com.actyx.dev",
            "com..*",
            " .actyx.*",
            "com. foo.*",
        ]
        .into_iter()
        .for_each(|app_domain| {
            let err = app_domain.parse::<AppDomain>().unwrap_err();
            assert!(matches!(err, InvalidAppDomainForm { .. }));
            assert_eq!(
                err.to_string(),
                format!("Required form '<tld>.<apex>.*'. Received: {}.", app_domain)
            );
        });
    }

    #[test]
    fn should_succeed() {
        let result: Result<AppDomain, InvalidAppDomainForm> = "com.actyx.*".parse();
        assert!(matches!(result, Ok(x) if x.to_string() == "com.actyx.*"));
    }

    #[test]
    fn valid_app_ids() {
        let app_domain: AppDomain = "com.actyx.*".parse().unwrap();
        vec!["com.actyx.test", "com.actyx.dev", "com.actyx.dev.test"]
            .into_iter()
            .for_each(|x| {
                let app_id = AppId::from_str(x).unwrap();
                assert!(app_domain.is_app_id_allowed(&app_id));
            });
    }

    #[test]
    fn invalid_app_ids() {
        let app_domain: AppDomain = "com.actyx.*".parse().unwrap();
        vec!["com.actyx", "com", "xxx.xxx"].into_iter().for_each(|x| {
            let app_id = AppId::from_str(x).unwrap();
            assert!(!app_domain.is_app_id_allowed(&app_id));
        });
    }
}
