//! Minimal case-conversion helpers.
//!
//! The emitters need `snake_case` and `PascalCase` conversions (ported from
//! club-unison, which used the `convert_case` crate). To keep `club-kdl-codegen`
//! dependency-free during Phase 1, these are reimplemented here with `std` only.
//!
//! The algorithm splits an identifier into words at:
//!
//! - existing separators (`_`, `-`, ` `, `:`, `/`, `.`),
//! - lower→upper transitions (`fooBar` → `foo`, `Bar`),
//! - upper-run boundaries before a final lowercase (`HTTPServer` → `HTTP`, `Server`).
//!
//! then re-joins the lowercased words in the requested style.
//!
//! The `:` / `/` / `.` separators let a wire-style schema name (`lane:delete`,
//! `process/spawn`) sanitize cleanly into a language identifier
//! (`LaneDelete` / `ProcessSpawn`) — characters valid in a KDL string but not
//! in a Rust / TypeScript identifier.

/// Split `s` into lowercase words on separator and case boundaries.
fn words(s: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut cur = String::new();
    let chars: Vec<char> = s.chars().collect();

    for (i, &c) in chars.iter().enumerate() {
        if matches!(c, '_' | '-' | ' ' | ':' | '/' | '.') {
            if !cur.is_empty() {
                out.push(std::mem::take(&mut cur));
            }
            continue;
        }

        if c.is_uppercase() && !cur.is_empty() {
            let prev = chars[i - 1];
            let next_lower = chars.get(i + 1).is_some_and(|n| n.is_lowercase());
            // Break on lower→upper, or on the last upper of an upper-run that
            // precedes a lowercase (`HTTPServer` → `HTTP` | `Server`).
            if prev.is_lowercase() || (prev.is_uppercase() && next_lower) {
                out.push(std::mem::take(&mut cur));
            }
        }

        cur.push(c.to_ascii_lowercase());
    }

    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// Convert `s` to `snake_case`.
pub fn to_snake_case(s: &str) -> String {
    words(s).join("_")
}

/// Convert `s` to `PascalCase`.
pub fn to_pascal_case(s: &str) -> String {
    words(s)
        .into_iter()
        .map(|w| {
            let mut ch = w.chars();
            match ch.next() {
                Some(first) => first.to_ascii_uppercase().to_string() + ch.as_str(),
                None => String::new(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snake_case_basics() {
        assert_eq!(to_snake_case("fooBar"), "foo_bar");
        assert_eq!(to_snake_case("FooBar"), "foo_bar");
        assert_eq!(to_snake_case("foo_bar"), "foo_bar");
        assert_eq!(to_snake_case("foo-bar"), "foo_bar");
        assert_eq!(to_snake_case("ping"), "ping");
    }

    #[test]
    fn snake_case_upper_runs() {
        assert_eq!(to_snake_case("HTTPServer"), "http_server");
        assert_eq!(to_snake_case("userID"), "user_id");
    }

    #[test]
    fn pascal_case_basics() {
        assert_eq!(to_pascal_case("foo_bar"), "FooBar");
        assert_eq!(to_pascal_case("foo-bar"), "FooBar");
        assert_eq!(to_pascal_case("fooBar"), "FooBar");
        assert_eq!(to_pascal_case("ping"), "Ping");
        assert_eq!(to_pascal_case("ping-pong"), "PingPong");
    }

    #[test]
    fn wire_name_separators_sanitize_to_identifiers() {
        // `:` / `/` / `.` are valid in a KDL string but not in a Rust / TS
        // identifier — they must split words like `_` / `-` / ` `.
        assert_eq!(to_pascal_case("lane:delete"), "LaneDelete");
        assert_eq!(to_pascal_case("process:toggle"), "ProcessToggle");
        assert_eq!(to_pascal_case("process/spawn"), "ProcessSpawn");
        assert_eq!(to_pascal_case("vp.sidebar"), "VpSidebar");
        assert_eq!(to_snake_case("lane:delete"), "lane_delete");
    }
}
