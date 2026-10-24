// SPDX-License-Identifier: LGPL-3.0-only
// Copyright (C) 2026 Manolo-dl
//
// Based on tosu v4.26.3 (https://github.com/tosuapp/tosu):
//   packages/tsprocess/lib/memory/memory.h
//   packages/tsprocess/src/process.ts
// Copyright (C) 2023-2026 Mikhail Babynichev and contributors.
// Modified in October 2026: ported from C++/TypeScript to Rust.

//! Byte patterns like `"8B CE 83 3D ?? ?? ?? ?? 00"`.

pub struct Pattern {
    /// `None` is a wildcard (`??`).
    bytes: Vec<Option<u8>>,
}

impl Pattern {
    pub fn parse(text: &str) -> Self {
        let bytes = text
            .split_whitespace()
            .map(|b| match b {
                "??" => None,
                hex => Some(u8::from_str_radix(hex, 16).expect("invalid byte in pattern")),
            })
            .collect();
        Self { bytes }
    }

    pub fn find_in(&self, haystack: &[u8]) -> Option<usize> {
        if self.bytes.is_empty() || haystack.len() < self.bytes.len() {
            return None;
        }

        // Use the first non-wildcard byte as an anchor to skip ahead quickly.
        let (anchor_idx, anchor) = self
            .bytes
            .iter()
            .enumerate()
            .find_map(|(i, b)| b.map(|b| (i, b)))?;

        let last_start = haystack.len() - self.bytes.len();
        let mut start = 0;
        while start <= last_start {
            let window = &haystack[start + anchor_idx..=last_start + anchor_idx];
            let pos = window.iter().position(|&b| b == anchor)?;
            let candidate = start + pos;

            let matches = self
                .bytes
                .iter()
                .zip(&haystack[candidate..])
                .all(|(p, &h)| p.is_none_or(|p| p == h));
            if matches {
                return Some(candidate);
            }
            start = candidate + 1;
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_with_wildcards() {
        let p = Pattern::parse("A1 ?? ?? 89 46");
        let data = [0x00, 0xA1, 0x11, 0x22, 0x89, 0x46, 0x00];
        assert_eq!(p.find_in(&data), Some(1));
    }

    #[test]
    fn leading_wildcard() {
        let p = Pattern::parse("?? 89 46");
        let data = [0x89, 0x46, 0x10, 0x89, 0x46];
        assert_eq!(p.find_in(&data), Some(2));
    }

    #[test]
    fn not_found() {
        let p = Pattern::parse("DE AD");
        assert_eq!(p.find_in(&[0xDE, 0x00, 0xAD]), None);
    }
}
