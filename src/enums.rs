//! Human-readable names for osu! stable numeric values.

pub fn ruleset(mode: i32) -> &'static str {
    match mode {
        0 => "osu",
        1 => "taiko",
        2 => "fruits",
        3 => "mania",
        _ => "?",
    }
}

pub fn ranked_status(status: i32) -> &'static str {
    match status {
        1 => "notSubmitted",
        2 => "pending",
        4 => "ranked",
        5 => "approved",
        6 => "qualified",
        7 => "loved",
        _ => "unknown",
    }
}
