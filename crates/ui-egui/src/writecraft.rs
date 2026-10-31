//! WriteCraft's trims to the WordCraft interface.
//!
//! The apps turn them on at start ([`set_trimmed`]); a `WordApp` built without that (the tests
//! inherited from WordCraft, the examples) keeps WordCraft's full interface, so those tests stay
//! as upstream has them and merges stay small. The trims only hide controls: every command still
//! runs from shortcuts, command search, the control channel and MCP.

use std::cell::Cell;

thread_local! {
    // Per thread: the UI runs on one thread, and each test's harness on its own.
    static TRIMMED: Cell<bool> = const { Cell::new(false) };
}

/// Turns WriteCraft's trims on or off for the UI on this thread.
pub fn set_trimmed(on: bool) {
    TRIMMED.with(|t| t.set(on));
}

/// Whether WriteCraft's trims are on for the UI on this thread.
pub fn trimmed() -> bool {
    TRIMMED.with(Cell::get)
}

/// WriteCraft's app settings, applied after the saved preferences: no spell checking (the
/// proofing tools have no French).
pub fn apply_app_defaults(app: &mut crate::WordApp) {
    app.session.view.proofing = false;
    app.session.prefs.check_spelling = false;
    app.ui.check_spelling = false;
}

/// The built-in ribbon tabs left off the strip.
pub const HIDDEN_TABS: [&str; 8] = ["Insert", "Draw", "Design", "Layout", "References", "Mailings", "Developer", "Zotero"];

/// Whether the strip leaves out the built-in tab `name`.
pub(crate) fn hides_tab(name: &str) -> bool {
    trimmed() && HIDDEN_TABS.contains(&name)
}

/// Marks the built-in tabs WriteCraft leaves out as hidden (custom tabs keep their state).
pub(crate) fn trim_tabs(tabs: &mut [wordcraft_engine::ribbon::TabEntry]) {
    for t in tabs.iter_mut().filter(|t| !t.custom && hides_tab(&t.name)) {
        t.hidden = true;
    }
}

/// Ribbon commands whose buttons and KeyTips are left out: proofing, the English thesaurus and
/// the language tools (spell checking is off).
pub const HIDDEN_COMMANDS: [&str; 4] = ["review.spelling", "review.thesaurus", "review.translate", "review.language"];

/// The Review groups left off the ribbon.
pub const HIDDEN_REVIEW_GROUPS: [&str; 1] = ["Language"];

/// Whether the Review tab leaves out the group titled `title`.
pub(crate) fn hides_review_group(title: &str) -> bool {
    trimmed() && HIDDEN_REVIEW_GROUPS.contains(&title)
}

/// Whether the ribbon leaves out the button that runs `id`.
pub(crate) fn hides_command(id: &str) -> bool {
    trimmed() && HIDDEN_COMMANDS.contains(&id)
}

/// Whether the group titled `title` loses its dialog launcher (Styles: no styles pane).
pub(crate) fn hides_launcher(title: &str) -> bool {
    trimmed() && title == "Styles"
}

/// The Home groups left off the ribbon.
pub const HIDDEN_HOME_GROUPS: [&str; 4] = ["Font", "Paragraph", "Voice", "Editor"];

/// Whether the Home tab leaves out the group titled `title`.
pub(crate) fn hides_home_group(title: &str) -> bool {
    trimmed() && HIDDEN_HOME_GROUPS.contains(&title)
}

/// The Home KeyTips that stay with the trims: those of the groups still shown.
pub(crate) const HOME_KEYTIPS: [&str; 7] = ["Paste", "Cut", "Copy", "Format Painter", "Find", "Replace", "Select"];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Services, WordApp};
    use egui::accesskit::Role;
    use egui_kittest::kittest::Queryable;
    use wordcraft_engine::Session;

    fn home_harness() -> egui_kittest::Harness<'static, WordApp> {
        let app = WordApp::new(Session::new(wordcraft_doc::Document::from_text("Text")), Services::default());
        egui_kittest::Harness::builder().with_size(egui::vec2(1440.0, 900.0)).build_ui_state(
            |ui, app: &mut WordApp| {
                let ctx = ui.ctx().clone();
                app.logic(&ctx);
                app.ui(ui);
            },
            app,
        )
    }

    #[test]
    fn home_leaves_out_font_paragraph_voice_and_editor() {
        set_trimmed(true);
        let mut h = home_harness();
        h.run_steps(4);
        for gone in [(Role::TextInput, "Font"), (Role::Button, "Bold"), (Role::Button, "Align Left"), (Role::Button, "Dictate")] {
            assert!(h.query_by_role_and_label(gone.0, tl!(gone.1)).is_none(), "{} is gone", gone.1);
        }
        for kept in ["Cut", "Copy", "Replace"] {
            assert!(h.query_by_label(tl!(kept)).is_some(), "{kept} stays");
        }
        // The commands still run.
        assert!(h.state_mut().run("format.bold", serde_json::json!({})).is_ok());
        set_trimmed(false);
    }

    #[test]
    fn without_the_trims_home_is_wordcrafts() {
        set_trimmed(false);
        let mut h = home_harness();
        h.run_steps(4);
        assert!(h.query_by_role_and_label(Role::TextInput, tl!("Font")).is_some());
    }

    #[test]
    fn the_strip_leaves_out_the_hidden_tabs() {
        set_trimmed(true);
        let mut h = home_harness();
        h.run_steps(4);
        for tab in HIDDEN_TABS {
            assert!(h.query_by_label(tl!(tab)).is_none(), "{tab} is gone");
        }
        for tab in ["File", "Home", "Review", "View", "Help"] {
            assert!(h.query_by_label(tl!(tab)).is_some(), "{tab} stays");
        }
        let badges = crate::keytips::badge_tabs(h.state());
        assert!(HIDDEN_TABS.iter().all(|t| !badges.contains(t)), "no KeyTips for hidden tabs: {badges:?}");
        set_trimmed(false);
    }

    #[test]
    fn app_defaults_turn_spell_checking_off() {
        let mut app = WordApp::new(Session::new(wordcraft_doc::Document::from_text("Text")), Services::default());
        assert!(app.session.view.proofing, "WordCraft checks spelling");
        apply_app_defaults(&mut app);
        assert!(!app.session.view.proofing && !app.session.prefs.check_spelling && !app.ui.check_spelling);
    }

    #[test]
    fn no_spelling_button_and_no_styles_launcher() {
        set_trimmed(true);
        let tips: Vec<&str> = crate::keytips::controls_public("Review").into_iter().map(|c| c.tip).collect();
        for gone in ["Spelling & Grammar", "Thesaurus", "Translate", "Language"] {
            assert!(!tips.contains(&gone), "{gone}: {tips:?}");
        }
        assert!(tips.contains(&"Read Aloud"), "{tips:?}");
        assert!(hides_command("review.spelling") && hides_launcher("Styles") && !hides_launcher("Clipboard"));
        let mut h = home_harness();
        h.state_mut().run("ui.tab", serde_json::json!({"tab": "Review"})).unwrap();
        h.run_steps(4);
        assert!(h.query_by_label(tl!("Thesaurus")).is_none() && h.query_by_label(tl!("Translate")).is_none());
        assert!(h.query_by_label(tl!("Word Count")).is_some(), "the rest of Proofing stays");
        set_trimmed(false);
        assert!(!hides_command("review.spelling") && !hides_launcher("Styles"));
    }

    #[test]
    fn home_keytips_follow_the_trims() {
        set_trimmed(true);
        let tips: Vec<String> = crate::keytips::controls_public("Home").into_iter().map(|c| c.tip.to_string()).collect();
        assert_eq!(tips, HOME_KEYTIPS);
        set_trimmed(false);
        assert!(crate::keytips::controls_public("Home").len() > HOME_KEYTIPS.len());
    }
}
