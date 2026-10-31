//! WriteCraft's defaults for new blank documents: French text, and Normal and Heading 1 set
//! for writing (Georgia, justified body, first-line indents, 1.5 line spacing).
//!
//! The apps turn them on ([`crate::Session::writecraft_defaults`]); a session without them (the
//! tests inherited from WordCraft, the CLI, MCP) keeps WordCraft's Normal template.

use wordcraft_doc::Document;
use wordcraft_doc::props::{Align, LineSpacing};

/// The language new documents are written in.
pub const LANGUAGE: &str = "fr-FR";
/// The body and heading font. A system font on macOS and Windows; elsewhere the font
/// substitution picks an open serif.
pub const FONT: &str = "Georgia";
/// Points in a centimetre.
const CM: f32 = 72.0 / 2.54;

/// Applies WriteCraft's defaults to a new blank document's styles.
pub fn apply_defaults(doc: &mut Document) {
    let styles = &mut doc.styles;
    styles.default_chr.lang = Some(LANGUAGE.into());
    if let Some(normal) = styles.get_mut("Normal") {
        normal.chr.font = Some(FONT.into());
        normal.chr.size = Some(14.0);
        normal.chr.color = None;
        normal.para.align = Some(Align::Justify);
        normal.para.indent_first = Some(0.5 * CM);
        normal.para.space_before = Some(0.0);
        normal.para.space_after = Some(6.0);
        normal.para.line_spacing = Some(LineSpacing::Multiple(1.5));
    }
    if let Some(h1) = styles.get_mut("Heading1") {
        h1.chr.font = Some(FONT.into());
        h1.chr.bold = Some(true);
        h1.chr.size = Some(20.0);
        h1.chr.color = None;
        h1.para.align = Some(Align::Left);
        h1.para.indent_first = Some(0.5 * CM);
        h1.para.space_before = Some(24.0);
        h1.para.space_after = Some(12.0);
        h1.para.line_spacing = Some(LineSpacing::Multiple(1.5));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Session;
    use serde_json::json;

    #[test]
    fn new_documents_follow_the_writecraft_defaults_only_when_on() {
        let mut s = Session::new(Document::new());
        s.run("file.new", &json!({})).unwrap();
        assert_ne!(s.doc.styles.default_chr.lang.as_deref(), Some(LANGUAGE), "off: WordCraft's Normal template");
        s.writecraft_defaults = true;
        s.run("file.new", &json!({})).unwrap();
        assert_eq!(s.doc.styles.default_chr.lang.as_deref(), Some(LANGUAGE));
        let st = s.run("format.state", &json!({})).unwrap();
        assert_eq!(st["font"], FONT);
        assert_eq!(st["size"], 14.0);
        assert_eq!(st["color"], "auto");
        let normal = s.doc.styles.get("Normal").unwrap();
        assert_eq!(normal.para.align, Some(Align::Justify));
        assert!((normal.para.indent_first.unwrap() - 14.17).abs() < 0.01);
        assert_eq!((normal.para.space_before, normal.para.space_after), (Some(0.0), Some(6.0)));
        assert_eq!(normal.para.line_spacing, Some(LineSpacing::Multiple(1.5)));
        let h1 = s.doc.styles.get("Heading1").unwrap();
        assert_eq!((h1.chr.font.as_deref(), h1.chr.bold, h1.chr.size, h1.chr.color), (Some(FONT), Some(true), Some(20.0), None));
        assert_eq!((h1.para.align, h1.para.space_before, h1.para.space_after), (Some(Align::Left), Some(24.0), Some(12.0)));
        assert_eq!(h1.para.line_spacing, Some(LineSpacing::Multiple(1.5)));
    }
}
