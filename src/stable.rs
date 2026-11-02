// SPDX-License-Identifier: LGPL-3.0-only
// Copyright (C) 2026 Manolo-dl
//
// Based on tosu v4.26.3 (https://github.com/tosuapp/tosu):
//   packages/tosu/src/memory/stable.ts
// Copyright (C) 2023-2026 Mikhail Babynichev and contributors.
// Modified in October-November 2026: ported from TypeScript to Rust; patterns
// are resolved lazily instead of all being required up front.

//! osu! stable memory signatures and the addresses resolved from them.

use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use crate::pattern::Pattern;
use crate::process::Process;
use crate::{Error, Result};

/// A memory signature. The discriminant indexes [`PATTERNS`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Sig {
    Base,
    PlayTime,
    ChatChecker,
    SkinData,
    SettingsClass,
    Configuration,
    Bindings,
    Rulesets,
    CanRunSlowly,
    Status,
    MenuMods,
    AudioLength,
    UserProfile,
    LoginStatus,
    SpectatingUser,
    GameTime,
    TourneyChat,
    ManiaSpeed,
}

struct SigInfo {
    name: &'static str,
    signature: &'static str,
    /// Added to the match address.
    offset: i64,
    /// Searched when attaching. The others only exist in some clients or after
    /// some code has run, so they are searched the first time they're needed.
    eager: bool,
}

const PATTERNS: [SigInfo; 18] = [
    SigInfo { name: "base", signature: "F8 01 74 04 83 65", offset: 0, eager: true },
    SigInfo { name: "playTime", signature: "5E 5F 5D C3 A1 ?? ?? ?? ?? 89 ?? 04", offset: 0, eager: true },
    SigInfo { name: "chatChecker", signature: "8B CE 83 3D ?? ?? ?? ?? 00 75 ?? 80", offset: 0x4, eager: true },
    SigInfo { name: "skinData", signature: "74 2C 85 FF 75 28 A1 ?? ?? ?? ?? 8D 15", offset: 0, eager: true },
    SigInfo { name: "settingsClass", signature: "83 E0 20 85 C0 7E 2F", offset: 0, eager: true },
    SigInfo {
        name: "configuration",
        signature: "8D 45 EC 50 8B 0D ?? ?? ?? ?? 8B D7 39 09 E8 ?? ?? ?? ?? 85 C0 74 ?? 8B 4D EC",
        offset: 0x6,
        eager: true,
    },
    SigInfo {
        name: "bindings",
        signature: "8D 7D D0 B9 08 00 00 00 33 C0 F3 AB 8B CE 89 4D DC B9",
        offset: 0x2a,
        eager: true,
    },
    SigInfo { name: "rulesets", signature: "7D 15 A1 ?? ?? ?? ?? 85 C0", offset: 0, eager: true },
    SigInfo { name: "canRunSlowly", signature: "55 8B EC 80 3D ?? ?? ?? ?? 00 75 26 80 3D", offset: 0, eager: true },
    SigInfo { name: "status", signature: "48 83 F8 04 73 1E", offset: -0x4, eager: true },
    SigInfo { name: "menuMods", signature: "C8 FF ?? ?? ?? ?? ?? 81 0D ?? ?? ?? ?? ?? 08 00 00", offset: 0x9, eager: true },
    SigInfo { name: "audioLength", signature: "55 8B EC 83 EC 08 A1 ?? ?? ?? ?? 85 C0", offset: 0x7, eager: true },
    SigInfo { name: "userProfile", signature: "FF 15 ?? ?? ?? ?? A1 ?? ?? ?? ?? 8B 48 54 33 D2", offset: 0x7, eager: true },
    SigInfo { name: "rawLoginStatus", signature: "B8 0B 00 00 8B 35", offset: -0xb, eager: true },
    SigInfo { name: "spectatingUser", signature: "8B 0D ?? ?? ?? ?? 85 C0 74 05 8B 50 30", offset: -0x4, eager: true },
    SigInfo { name: "gameTime", signature: "A1 ?? ?? ?? ?? 89 46 04 8B D6 E8", offset: 0x1, eager: true },
    SigInfo { name: "tourneyChat", signature: "A1 ?? ?? ?? ?? 89 45 F0 8B D1 85 C9 75", offset: 0, eager: false },
    SigInfo {
        name: "maniaSpeed",
        signature: "a3 ?? ?? ?? ?? eb ?? dd 45 08 db 5d e8 8b 45 e8 83 f8 28",
        offset: 0,
        eager: false,
    },
];

/// Signatures [`OsuReader::attach`](crate::OsuReader::attach) requires, as in
/// version 0.1.
const REQUIRED: [Sig; 2] = [Sig::Base, Sig::SettingsClass];

/// Wait after a search that didn't find the requested signature. It doubles
/// with each such search, up to [`MAX_RESCAN_DELAY`]: a search reads the
/// game's code memory (around 0.1 s), so it must not run on every poll.
const FIRST_RESCAN_DELAY: Duration = Duration::from_secs(5);
const MAX_RESCAN_DELAY: Duration = Duration::from_secs(60);

pub(crate) struct Addresses {
    found: [OnceLock<u64>; PATTERNS.len()],
    rescan: Mutex<Rescan>,
}

struct Rescan {
    /// No search before this time.
    not_before: Instant,
    delay: Duration,
}

impl Addresses {
    pub(crate) fn scan(process: &Process) -> Result<Self> {
        let addresses = Self {
            found: [const { OnceLock::new() }; PATTERNS.len()],
            rescan: Mutex::new(Rescan { not_before: Instant::now(), delay: FIRST_RESCAN_DELAY }),
        };
        addresses.search(process, |info| info.eager, false)?;

        for sig in REQUIRED {
            if addresses.found[sig as usize].get().is_none() {
                return Err(Error::PatternNotFound { name: PATTERNS[sig as usize].name });
            }
        }
        Ok(addresses)
    }

    /// Address of `sig`, searching for it (and any other missing one) if it
    /// hasn't been found yet and the wait since the last search has passed.
    pub(crate) fn get(&self, process: &Process, sig: Sig) -> Result<u64> {
        if let Some(&address) = self.found[sig as usize].get() {
            return Ok(address);
        }

        {
            let mut rescan = self.rescan.lock().unwrap_or_else(|e| e.into_inner());
            // Another thread may have found it while this one waited for the lock.
            if self.found[sig as usize].get().is_none() && Instant::now() >= rescan.not_before {
                // Signatures are code, so only executable memory is searched again.
                // Errors mean the memory can't be read at all; reported below as not found.
                let _ = self.search(process, |_| true, true);
                if self.found[sig as usize].get().is_none() {
                    rescan.not_before = Instant::now() + rescan.delay;
                    rescan.delay = (rescan.delay * 2).min(MAX_RESCAN_DELAY);
                }
            }
        }

        self.found[sig as usize].get().copied().ok_or(Error::PatternNotFound { name: PATTERNS[sig as usize].name })
    }

    /// Searches for the missing signatures accepted by `filter` in one pass.
    fn search(&self, process: &Process, filter: impl Fn(&SigInfo) -> bool, code_only: bool) -> Result<()> {
        let wanted: Vec<usize> =
            (0..PATTERNS.len()).filter(|&i| self.found[i].get().is_none() && filter(&PATTERNS[i])).collect();
        if wanted.is_empty() {
            return Ok(());
        }

        let patterns: Vec<Pattern> = wanted.iter().map(|&i| Pattern::parse(PATTERNS[i].signature)).collect();
        let results = process.scan_batch(&patterns, code_only)?;

        for (&i, address) in wanted.iter().zip(results) {
            if let Some(address) = address {
                let _ = self.found[i].set(address.wrapping_add_signed(PATTERNS[i].offset));
            }
        }
        Ok(())
    }
}

/// `[[Rulesets - 0xB] + 0x4]`: the object of the current screen (gameplay,
/// results, tourney manager...). Zero while switching screens.
pub(crate) fn ruleset(p: &Process, a: &Addresses) -> Result<u64> {
    let rulesets = a.get(p, Sig::Rulesets)?;
    p.read_ptr(p.read_ptr(rulesets - 0xb)? + 0x4)
}

/// Mods are stored as two integers XORed together.
pub(crate) fn read_mods(p: &Process, mods_object: u64) -> Result<u32> {
    Ok(p.read_u32(mods_object + 0xc)? ^ p.read_u32(mods_object + 0x8)?)
}

