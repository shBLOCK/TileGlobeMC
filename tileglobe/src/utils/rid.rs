#![allow(unused)]

use crate::utils::MINECRAFT;
use alloc::borrow::{Cow, ToOwned};
use alloc::string::String;
use core::fmt::{Display, Formatter};
use core::marker::PhantomData;
use serde::de::{Error, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct RID<'a> {
    pub namespace: Cow<'a, str>,
    pub path: Cow<'a, str>,
}

impl<'a> RID<'a> {
    pub const fn new(namespace: &'a str, path: &'a str) -> Self {
        Self {
            namespace: Cow::Borrowed(namespace),
            path: Cow::Borrowed(path),
        }
    }

    pub const fn new_owned(namespace: String, path: String) -> Self {
        Self {
            namespace: Cow::Owned(namespace),
            path: Cow::Owned(path),
        }
    }

    pub fn new_checked(namespace: &'a str, path: &'a str) -> Result<Self, RIDError> {
        let rid = Self::new(namespace, path);
        rid.validate()?;
        Ok(rid)
    }

    pub fn new_owned_checked(namespace: String, path: String) -> Result<Self, RIDError> {
        let rid = Self::new_owned(namespace, path);
        rid.validate()?;
        Ok(rid)
    }

    pub fn into_owned(self) -> RID<'static> {
        RID::new_owned(self.namespace.into_owned(), self.path.into_owned())
    }

    pub fn validate(&self) -> Result<(), RIDError> {
        if !Self::is_valid_namespace(&self.namespace) {
            return Err(RIDError::InvalidNamespace);
        }
        if !Self::is_valid_path(&self.path) {
            return Err(RIDError::InvalidPath);
        }
        Ok(())
    }

    pub fn is_valid_namespace(value: &str) -> bool {
        !value.is_empty()
            && value.chars().all(|c| {
                c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-' || c == '.'
            })
    }

    pub fn is_valid_path(value: &str) -> bool {
        !value.is_empty()
            && value.chars().all(|c| {
                c.is_ascii_lowercase()
                    || c.is_ascii_digit()
                    || c == '_'
                    || c == '-'
                    || c == '.'
                    || c == '/'
            })
    }
}

impl Display for RID<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}:{}", self.namespace, self.path)
    }
}

impl<'a> TryFrom<&'a str> for RID<'a> {
    type Error = RIDError;

    fn try_from(s: &'a str) -> Result<Self, Self::Error> {
        let namespace: &str;
        let path: &str;
        match s.split_once(':') {
            Some((a, b)) => {
                namespace = a;
                path = b;
            }
            None => {
                namespace = MINECRAFT;
                path = s;
            }
        }
        Self::new_checked(namespace, path)
    }
}

impl From<RID<'_>> for String {
    fn from(value: RID) -> Self {
        let mut string = String::with_capacity(value.namespace.len() + 1 + value.path.len());
        string += &value.namespace;
        string += ":";
        string += &value.path;
        string
    }
}

#[derive(Debug, derive_more::Display)]
#[display("{self:?}")]
pub enum RIDError {
    InvalidNamespace,
    InvalidPath,
}

#[cfg(feature = "std")]
mod _std {
    extern crate std;
    use super::*;
    use std::path::PathBuf;

    impl From<&RID<'_>> for PathBuf {
        fn from(value: &RID) -> Self {
            [&*value.namespace, &*value.path].iter().collect()
        }
    }
}

impl Serialize for RID<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(&format_args!("{}:{}", &*self.namespace, &*self.path))
    }
}

impl<'de: 'a, 'a> Deserialize<'de> for RID<'a> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct RIDVisitor<'a>(PhantomData<&'a ()>);

        impl<'de: 'a, 'a> Visitor<'de> for RIDVisitor<'a> {
            type Value = RID<'a>;

            fn expecting(&self, formatter: &mut Formatter) -> core::fmt::Result {
                write!(
                    formatter,
                    "a Minecraft ResourceLocation, e.g. \"minecraft:stone\" or \"stone\"."
                )
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: Error,
            {
                Ok(RID::try_from(v).map_err(E::custom)?.into_owned())
            }

            fn visit_borrowed_str<E>(self, v: &'de str) -> Result<Self::Value, E>
            where
                E: Error,
            {
                RID::try_from(v).map_err(E::custom)
            }
        }

        deserializer.deserialize_str(RIDVisitor(PhantomData))
    }
}

impl RID<'static> {
    pub fn de_owned<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(RID::deserialize(deserializer)?.into_owned())
    }
}
