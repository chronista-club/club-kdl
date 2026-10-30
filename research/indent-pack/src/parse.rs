// Generated research copy; see generate.py.
/// ASCII bytes that may appear in an identifier string.
const fn ident_table() -> [bool; 128] {
    let mut t = [false; 128];
    let mut i = 0x21;
    while i < 0x7F {
        t[i] = true;
        i += 1;
    }
    let banned = b"\\/(){};[]\"#=";
    let mut j = 0;
    while j < banned.len() {
        t[banned[j] as usize] = false;
        j += 1;
    }
    t
}
pub(crate) static IDENT_ASCII: [bool; 128] = ident_table();

#[inline]
pub(crate) fn is_unicode_space(c: char) -> bool {
    matches!(
        c,
        '\t' | ' ' | '\u{00A0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200A}' | '\u{202F}' | '\u{205F}' | '\u{3000}'
    )
}

#[inline]
pub(crate) fn is_newline_char(c: char) -> bool {
    matches!(
        c,
        '\r' | '\n' | '\u{000B}' | '\u{000C}' | '\u{0085}' | '\u{2028}' | '\u{2029}'
    )
}

#[inline]
pub(crate) fn is_disallowed(c: char) -> bool {
    matches!(
        c,
        '\u{0000}'..='\u{0008}'
            | '\u{000E}'..='\u{001F}'
            | '\u{007F}'
            | '\u{200E}'..='\u{200F}'
            | '\u{202A}'..='\u{202E}'
            | '\u{2066}'..='\u{2069}'
            | '\u{FEFF}'
    )
}

#[inline]
pub(crate) fn is_ident_char(c: char) -> bool {
    if c.is_ascii() {
        IDENT_ASCII[c as usize]
    } else {
        !is_unicode_space(c) && !is_newline_char(c) && !is_disallowed(c)
    }
}
