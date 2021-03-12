use derive_more::{AsRef, Display, From, Into};
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use std::fmt;
use std::fmt::{Display, Error as FmtError, Formatter};
use std::path::Path;
use std::{borrow::Borrow, ops::Sub, str::FromStr};

pub mod admin_protocol;
pub mod errors;
pub mod logs;
mod util;

pub use admin_protocol::*;
pub use errors::*;
pub use logs::*;

pub const ACTYXOS_ID: &str = "com.actyx.os";

/// The api response to ax-cli
#[derive(Debug, Serialize, Deserialize)]
pub struct ResponseMessage {
    pub message: String,
}
impl ResponseMessage {
    pub fn new(message: String) -> ResponseMessage {
        ResponseMessage { message }
    }
}

impl fmt::Display for ResponseMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug, Eq, PartialOrd, Ord, From, Into, AsRef, Display)]
pub struct AppId(pub String);
impl AppId {
    pub fn is_actyxos(&self) -> bool {
        self.0 == ACTYXOS_ID
    }
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}
impl From<&str> for AppId {
    fn from(str: &str) -> Self {
        Self(str.to_string())
    }
}
impl<'a> Into<&'a Path> for &'a AppId {
    fn into(self) -> &'a Path {
        Path::new(self.0.as_str())
    }
}
impl Into<axossettings::Scope> for AppId {
    fn into(self) -> axossettings::Scope {
        self.0.parse().expect("AppId must not be empty!")
    }
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug, Eq, PartialOrd, Ord, From, Into, AsRef, Display)]
pub struct AppVersion(pub String);
impl AppVersion {
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}
impl From<&str> for AppVersion {
    fn from(str: &str) -> Self {
        Self(str.to_string())
    }
}

#[derive(Deserialize, PartialEq, Clone, Debug, From, Into, AsRef, Display)]
pub struct NodeName(pub String);
/// This macro declares an enum where the variant names match their string representation
///
/// A variant without the `current()` function could be considered in a more general context.
macro_rules! string_enum {
    ($vis:vis enum $id:ident {$(#[$attr:meta] $n:ident,)*} $err:literal) => {
        #[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, PartialOrd, Ord, Eq)]
        #[allow(non_camel_case_types)]
        $vis enum $id {
            $($n,)*
            any
        }
        impl $id {
            #[allow(unreachable_code)]
            pub fn current() -> Self {
                $(
                    #[$attr]
                    return $id::$n;
                )*
                unreachable!($err);
            }
            pub fn all() -> &'static[$id] {
                static X: &[$id] = &[
                    $(
                        $id::$n,
                    )*
                    $id::any
                ];
                X
            }
        }
        impl Display for $id {
            fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
                f.write_str(match self {
                    $(
                        $id::$n => stringify!($n),
                    )*
                    $id::any => "any"
                })
            }
        }
        impl FromStr for $id {
            type Err = String;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Ok(match s {
                    $(
                        stringify!($n) => $id::$n,
                    )*
                    "any" => $id::any,
                    arch => {
                        return Err(format!("{}: {}", $err, arch))
                    }
                })
            }
        }
        impl TryFrom<String> for $id {
            type Error = String;
            fn try_from(s: String) -> Result<Self, Self::Error> {
                Self::from_str(&s)
            }
        }
    };
}

string_enum! {
    pub enum Arch {
        #[cfg(target_arch = "x86_64")] x86_64,
        #[cfg(target_arch = "aarch64")] aarch64,
        #[cfg(target_os = "android")] webview,
    }
    "Unsupported architecture"
}

string_enum! {
    pub enum OS {
        #[cfg(target_os = "linux")] linux,
        #[cfg(target_os = "windows")] windows,
        #[cfg(target_os = "macos")] macos,
        #[cfg(target_os = "android")] android,
    }
    "Unsupported OS"
}

impl<A: Borrow<Arch>> Sub<A> for OS {
    type Output = OsArch;
    fn sub(self, rhs: A) -> Self::Output {
        OsArch::new(self, *rhs.borrow())
    }
}

impl<A: Borrow<Arch>> Sub<A> for &OS {
    type Output = OsArch;
    fn sub(self, rhs: A) -> Self::Output {
        OsArch::new(*self, *rhs.borrow())
    }
}

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Serialize, Deserialize)]
#[serde(into = "String", try_from = "String")]
pub struct OsArch {
    os: OS,
    arch: Arch,
}

impl OsArch {
    fn new(os: OS, arch: Arch) -> Self {
        Self { os, arch }
    }
    pub fn is_supported(arch: Arch) -> bool {
        (Arch::current() == arch) // arch matches
        // is webview
         || (arch == Arch::webview && (OS::current() == OS::android || OS::current() == OS::windows))
    }
}

impl Display for OsArch {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}-{}", self.os, self.arch)
    }
}

impl Into<String> for OsArch {
    fn into(self) -> String {
        self.to_string()
    }
}

impl TryFrom<String> for OsArch {
    type Error = String;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        let mut parts = value.split('-');
        let os = parts.next().ok_or("OsArch cannot be empty")?;
        let arch = parts.next().ok_or("OsArch needs two parts separated by '-'")?;
        if parts.next().is_some() {
            return Err("OsArch cannot have more than two parts".into());
        }
        Ok(Self::new(OS::from_str(os)?, Arch::from_str(arch)?))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn os_arch_strings() {
        for os in OS::all() {
            let json = serde_json::to_string(os).unwrap();
            let os2: OS = serde_json::from_str(&json).unwrap();
            assert_eq!(*os, os2);
            for arch in Arch::all() {
                let json = serde_json::to_string(arch).unwrap();
                let arch2: Arch = serde_json::from_str(&json).unwrap();
                assert_eq!(*arch, arch2);

                let oa = os - arch;
                let s = oa.to_string();
                let json = serde_json::to_string(&oa).unwrap();
                assert_eq!(format!(r#""{}""#, s), json);
                let oa2: OsArch = serde_json::from_str(&json).unwrap();
                assert_eq!(oa, oa2);
            }
        }

        let oa: OsArch = serde_yaml::from_str("windows-x86_64").unwrap();
        assert_eq!(OS::windows - Arch::x86_64, oa);

        #[allow(clippy::op_ref)]
        {
            assert_eq!(&OS::any - Arch::any, OsArch::new(OS::any, Arch::any));
            assert_eq!(OS::any - &Arch::any, OsArch::new(OS::any, Arch::any));
            assert_eq!(&OS::any - &Arch::any, OsArch::new(OS::any, Arch::any));
        }
    }
}
