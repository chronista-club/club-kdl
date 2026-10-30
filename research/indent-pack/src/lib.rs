pub use club_kdl::{ToKdlValue, ast};

#[allow(dead_code)]
mod baseline;
#[allow(dead_code)]
mod no_indent;
mod parse;
#[allow(dead_code)]
mod single_line;

#[derive(Clone, Copy, Debug)]
pub enum Layout {
    Pretty,
    NoIndent,
    SingleLine,
}

pub fn write(doc: &ast::Document<'_>, layout: Layout) -> String {
    let mut out = String::new();
    write_into(doc, layout, &mut out);
    out
}

pub fn write_into(doc: &ast::Document<'_>, layout: Layout, out: &mut String) {
    match layout {
        Layout::Pretty => baseline::write_document(out, doc),
        Layout::NoIndent => no_indent::write_document(out, doc),
        Layout::SingleLine => single_line::write_document(out, doc),
    }
}
