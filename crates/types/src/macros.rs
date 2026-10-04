//! The `open_string_enum!` macro.
//!
//! In its own module so that `lib.rs` can declare its submodules at the top of the file
//! like every other crate. A `macro_rules!` macro is only in scope from its definition
//! onwards, so while this lived in `lib.rs` the `pub mod query;` line had to sit *below*
//! it, in the middle of the file, where no reader looks for a module declaration.
//!
//! The `pub(crate) use` at the bottom is what lets a sibling module write
//! `use crate::macros::open_string_enum;` instead of relying on `#[macro_use]` and
//! textual ordering. The macro's own body deliberately leaves `fmt`, `serde` and `TS`
//! unqualified, so a module invoking it imports those too — which every module that
//! derives serde on a struct is doing anyway.

macro_rules! open_string_enum {
    (
        $(#[$enum_meta:meta])*
        pub enum $name:ident {
            $(
                $(#[$variant_meta:meta])*
                $variant:ident => $wire:literal,
            )+
        }
    ) => {
        $(#[$enum_meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash, TS)]
        #[ts(type = "string")]
        pub enum $name {
            $(
                $(#[$variant_meta])*
                $variant,
            )+
            /// A value that this version of Geoquery does not name yet.
            Custom(String),
        }

        impl $name {
            /// Return the exact string representation used in JSON documents.
            #[must_use]
            pub fn as_str(&self) -> &str {
                match self {
                    $(Self::$variant => $wire,)+
                    Self::Custom(value) => value.as_str(),
                }
            }

            /// The wire table, written once. `from_wire` and `from_string` differ only in
            /// who owns the string they fall back to, so stating the mapping in each of
            /// them would be two copies that a new variant could be added to one of.
            fn named(value: &str) -> Option<Self> {
                match value {
                    $($wire => Some(Self::$variant),)+
                    _ => None,
                }
            }

            fn from_wire(value: &str) -> Self {
                Self::named(value).unwrap_or_else(|| Self::Custom(value.to_owned()))
            }

            fn from_string(value: String) -> Self {
                // `unwrap_or_else` rather than `unwrap_or`: the fallback moves `value`, and
                // building it eagerly would hand the allocation away on the path that does
                // not need it.
                Self::named(&value).unwrap_or_else(|| Self::Custom(value))
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                self.as_str()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(self.as_str())
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self::from_wire(value)
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self::from_string(value)
            }
        }

        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                serializer.serialize_str(self.as_str())
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                struct OpenEnumVisitor;

                impl Visitor<'_> for OpenEnumVisitor {
                    type Value = $name;

                    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                        formatter.write_str(concat!("a string containing a ", stringify!($name), " value"))
                    }

                    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
                    where
                        E: de::Error,
                    {
                        Ok($name::from_wire(value))
                    }

                    fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
                    where
                        E: de::Error,
                    {
                        Ok($name::from_string(value))
                    }
                }

                deserializer.deserialize_str(OpenEnumVisitor)
            }
        }
    };
}

pub(crate) use open_string_enum;
