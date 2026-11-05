// SPDX-License-Identifier: LGPL-3.0-only
// Copyright (C) 2026 Manolo-dl
//
// Based on tosu v4.26.3 (https://github.com/tosuapp/tosu):
//   packages/tsprocess/lib/memory/memory.h
//   packages/tsprocess/src/process.ts
// Copyright (C) 2023-2026 Mikhail Babynichev and contributors.
// Modified in October-November 2026: ported from C++/TypeScript to Rust; a
// pattern is searched for by its longest run of fixed bytes, with memchr.

//! Byte patterns like `"8B CE 83 3D ?? ?? ?? ?? 00"`.

use memchr::memmem::Finder;

pub struct Pattern {
    /// `None` is a wildcard (`??`).
    bytes: Vec<Option<u8>>,
    /// The longest run of fixed bytes, searched for first: with SIMD, and far
    /// rarer in the game's code than any one byte of it (`55` alone is the
    /// first byte of most functions). Empty when the pattern has none.
    needle: Finder<'static>,
    /// Where [`needle`](Self::needle) starts in the pattern.
    needle_at: usize,
}

impl Pattern {
    pub fn parse(text: &str) -> Self {
        let bytes: Vec<Option<u8>> = text
            .split_whitespace()
            .map(|b| match b {
                "??" => None,
                hex => Some(u8::from_str_radix(hex, 16).expect("invalid byte in pattern")),
            })
            .collect();

        // The first of the longest runs of fixed bytes.
        let (mut needle_at, mut needle_len, mut run_at) = (0, 0, 0);
        for (i, byte) in bytes.iter().enumerate() {
            if byte.is_none() {
                run_at = i + 1;
            } else if i + 1 - run_at > needle_len {
                (needle_at, needle_len) = (run_at, i + 1 - run_at);
            }
        }
        let needle: Vec<u8> = bytes[needle_at..needle_at + needle_len].iter().flatten().copied().collect();
        Self { needle: Finder::new(&needle).into_owned(), needle_at, bytes }
    }

    /// Where the first match in `haystack` starts.
    pub fn find_in(&self, haystack: &[u8]) -> Option<usize> {
        if self.needle.needle().is_empty() || haystack.len() < self.bytes.len() {
            return None;
        }

        let last_start = haystack.len() - self.bytes.len();
        // One occurrence at a time: `find_iter` skips the ones that overlap,
        // and a match can start inside a run that didn't match.
        let mut from = self.needle_at;
        while let Some(pos) = self.needle.find(&haystack[from..]) {
            let start = from + pos - self.needle_at;
            if start > last_start {
                return None;
            }
            if self.matches(&haystack[start..]) {
                return Some(start);
            }
            from += pos + 1;
        }
        None
    }

    /// Whether `haystack` starts with this pattern.
    fn matches(&self, haystack: &[u8]) -> bool {
        self.bytes.iter().zip(haystack).all(|(p, &h)| p.is_none_or(|p| p == h))
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

    #[test]
    fn match_inside_a_run_that_failed() {
        // The run `AA AA` is at 0, 1 and 2; only the one at 1 is followed by `BB`.
        let p = Pattern::parse("AA AA BB");
        assert_eq!(p.find_in(&[0xAA, 0xAA, 0xAA, 0xBB]), Some(1));
    }

    #[test]
    fn longest_run_after_wildcards() {
        let p = Pattern::parse("A1 ?? ?? ?? ?? 89 46 04 8B");
        let data = [0xA1, 0, 0, 0, 0, 0x89, 0x46, 0x04, 0x8B, 0xA1, 1, 2, 3, 4, 0x89, 0x46, 0x04, 0x8B];
        assert_eq!(p.find_in(&data), Some(0));
        assert_eq!(p.find_in(&data[1..]), Some(8));
    }

    #[test]
    fn only_wildcards_never_match() {
        assert_eq!(Pattern::parse("?? ??").find_in(&[1, 2, 3]), None);
    }

    /// The first start where every fixed byte matches, trying each one.
    fn reference(pattern: &[Option<u8>], haystack: &[u8]) -> Option<usize> {
        if pattern.iter().all(Option::is_none) || haystack.len() < pattern.len() {
            return None;
        }
        (0..=haystack.len() - pattern.len())
            .find(|&s| pattern.iter().zip(&haystack[s..]).all(|(p, &h)| p.is_none_or(|p| p == h)))
    }

    #[test]
    fn same_as_trying_every_start() {
        // Three byte values, so matches, near misses and overlaps are everywhere.
        let mut seed = 0x9E37_79B9_7F4A_7C15u64;
        let mut next = move || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        for _ in 0..20_000 {
            let haystack: Vec<u8> = (0..next() % 200).map(|_| [0xAA, 0xBB, 0xCC][(next() % 3) as usize]).collect();
            let text: Vec<String> = (0..1 + next() % 6)
                .map(|_| match next() % 4 {
                    0 => "??".to_string(),
                    i => format!("{:02X}", [0xAA, 0xBB, 0xCC][i as usize - 1]),
                })
                .collect();
            let pattern = Pattern::parse(&text.join(" "));
            assert_eq!(pattern.find_in(&haystack), reference(&pattern.bytes, &haystack), "{text:?} in {haystack:02X?}");
        }
    }
}
