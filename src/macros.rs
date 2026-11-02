// SPDX-License-Identifier: LGPL-3.0-only
// Copyright (C) 2026 Manolo-dl

/// Defines an enum backed by a raw `i32` from the game memory, with
/// `from_raw`, `raw`, `name`, `Display`, `Default` (the value `0`) and, with
/// the `serde` feature, (de)serialization as its name. Values the crate doesn't
/// know are kept in an `Other` variant.
macro_rules! raw_enum {
    (
        $(#[$meta:meta])*
        pub enum $name:ident {
            $(
                $(#[$vmeta:meta])*
                $variant:ident = $value:literal => $str:literal,
            )*
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        pub enum $name {
            $(
                $(#[$vmeta])*
                #[cfg_attr(feature = "serde", serde(rename = $str))]
                $variant,
            )*
            /// A value this crate doesn't know, kept as read.
            #[cfg_attr(feature = "serde", serde(rename = "other"))]
            Other(i32),
        }

        impl $name {
            /// Converts the raw value stored by the game.
            pub fn from_raw(value: i32) -> Self {
                match value {
                    $($value => Self::$variant,)*
                    other => Self::Other(other),
                }
            }

            /// Raw value used by the game.
            pub fn raw(self) -> i32 {
                match self {
                    $(Self::$variant => $value,)*
                    Self::Other(value) => value,
                }
            }

            /// camelCase name, the same used by tosu and by serde.
            pub fn name(self) -> &'static str {
                match self {
                    $(Self::$variant => $str,)*
                    Self::Other(_) => "other",
                }
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::from_raw(0)
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(self.name())
            }
        }
    };
}

pub(crate) use raw_enum;
