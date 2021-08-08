#![allow(clippy::upper_case_acronyms)]

use std::convert::TryFrom;
use std::fmt;
use std::fmt::{Display, Error as FmtError, Formatter};
use std::{borrow::Borrow, ops::Sub, str::FromStr};

/// This macro declares an enum where the variant names match their string representation
///
/// A variant without the `current()` function could be considered in a more general context.
macro_rules!  string_enum {
    ($vis:vis enum $id:ident {$(#[$attr:meta] $n:ident,)*} $err:literal) => {
        #[derive(Debug, Clone, Copy, PartialEq,  PartialOrd, Ord, Eq)]
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
        #[cfg(AX_ARCH = "i686")] x86,
        #[cfg(AX_ARCH = "x86_64")] x86_64,
        #[cfg(AX_ARCH = "aarch64")] aarch64,
        #[cfg(AX_ARCH = "android")] android,
        #[cfg(AX_ARCH = "arm")] arm,
        #[cfg(AX_ARCH = "armv7")] armv7,
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

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct OsArch {
    pub os: OS,
    pub arch: Arch,
}

impl OsArch {
    fn new(os: OS, arch: Arch) -> Self {
        Self { os, arch }
    }

    pub fn current() -> Self {
        OS::current() - Arch::current()
    }

    pub fn is_supported(arch: Arch) -> bool {
        Arch::current() == arch
    }

    pub fn all() -> Vec<Self> {
        [
            "android-any",
            "linux-aarch64",
            "linux-arm",
            "linux-armv7",
            "linux-x86_64",
            "macos-aarch64",
            "macos-x86_64",
            "windows-x86_64",
        ]
        .iter()
        .map(|x| TryFrom::try_from(x.to_string()).unwrap())
        .collect()
    }
}

impl Display for OsArch {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}-{}", self.os, self.arch)
    }
}

#[allow(clippy::from_over_into)]
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
