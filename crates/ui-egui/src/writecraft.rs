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

/// WriteCraft's own interface text, in English and French: craft-i18n has no WriteCraft rows.
const TEXTS: &[(&str, &str)] = &[
    ("About WriteCraft", "À propos de WriteCraft"),
    ("A distraction-free word processor for writers, written in Rust.", "Un traitement de texte épuré, pensé pour l'écriture, écrit en Rust."),
    (
        "Based on WordCraft, by the ArtCraft team and the WordCraft contributors.",
        "Basé sur WordCraft, de l'équipe ArtCraft et des contributeurs de WordCraft.",
    ),
    ("WriteCraft source code: github.com/ymauray/writecraft", "Code source de WriteCraft : github.com/ymauray/writecraft"),
    ("WordCraft source code: github.com/storytold/wordcraft", "Code source de WordCraft : github.com/storytold/wordcraft"),
    (
        "MIT OR Apache-2.0. Copyright (c) 2026 ArtCraft Team, the WordCraft contributors and the WriteCraft contributors.",
        "MIT OU Apache-2.0. Copyright (c) 2026 l'équipe ArtCraft, les contributeurs de WordCraft et ceux de WriteCraft.",
    ),
    // The Pomodoro tab and status.
    ("Timer", "Minuteur"),
    ("Durations", "Durées"),
    ("Start", "Démarrer"),
    ("Resume", "Reprendre"),
    ("Reset", "Réinitialiser"),
    ("Skip", "Passer"),
    ("Stop", "Arrêter"),
    ("Restart the current period", "Recommencer la période en cours"),
    ("Go to the next period", "Passer à la période suivante"),
    ("Stop and reset the cycle", "Arrêter et remettre le cycle à zéro"),
    ("Short break (min)", "Pause courte (min)"),
    ("Long break (min)", "Pause longue (min)"),
    ("Long break after (pomodoros)", "Pause longue après (pomodoros)"),
    ("Auto-start breaks", "Lancer les pauses automatiquement"),
    ("Auto-start pomodoros", "Lancer les pomodoros automatiquement"),
    ("Notify at the end", "Prévenir à la fin"),
    ("Sounds", "Sons"),
    ("Today: {n}", "Aujourd'hui : {n}"),
    ("Session: {n}", "Session : {n}"),
    ("Words written today, in every document", "Mots écrits aujourd'hui, tous documents confondus"),
    ("Words written in the current or last pomodoro", "Mots écrits pendant le pomodoro en cours ou le dernier"),
    ("No sound", "Aucun"),
    ("Beeps", "Bips"),
    ("Fun", "Rigolos"),
    ("Listen", "Écouter"),
    ("Play the start, break and end sounds", "Écouter les sons de début, de pause et de fin"),
    ("Short break", "Pause courte"),
    ("Long break", "Pause longue"),
    ("Paused", "En pause"),
    ("Click to start or pause the timer", "Cliquer pour démarrer ou mettre en pause le minuteur"),
    ("Pomodoro finished: time for a break.", "Pomodoro terminé : c'est l'heure de la pause."),
    ("Pomodoro finished: time for a long break.", "Pomodoro terminé : c'est l'heure de la pause longue."),
    ("Break over: back to work.", "Pause terminée : au travail !"),
];

/// `en` in the interface language: French when the interface is French, English otherwise.
pub(crate) fn text(en: &'static str) -> &'static str {
    if crate::i18n::current().code().starts_with("fr") { TEXTS.iter().find(|(e, _)| *e == en).map_or(en, |(_, fr)| *fr) } else { en }
}

/// The app's name in window titles: WriteCraft with the trims on, else WordCraft.
pub(crate) fn app_name() -> &'static str {
    if trimmed() { "WriteCraft" } else { "WordCraft" }
}

/// A dialog's window title: WriteCraft's About title and name with the trims on, else the
/// translated one.
pub(crate) fn dialog_title(title: &'static str) -> &'static str {
    match title {
        "About WordCraft" if trimmed() => text("About WriteCraft"),
        "WordCraft" if trimmed() => "WriteCraft",
        _ => tl!(title),
    }
}

/// WriteCraft's version: `WRITECRAFT_VERSION` from the release build (the tag), else the
/// WordCraft version it is built on.
pub(crate) fn version() -> &'static str {
    option_env!("WRITECRAFT_VERSION").filter(|v| !v.is_empty()).unwrap_or(env!("CARGO_PKG_VERSION"))
}

/// The About tab of the About dialog, for WriteCraft. The Contributors and Models tabs stay
/// WordCraft's: WriteCraft is built on their work.
pub(crate) fn about_ui(ui: &mut egui::Ui) {
    ui.label(egui::RichText::new("WriteCraft").font(crate::theme::semibold(22.0)));
    let build = option_env!("WORDCRAFT_BUILD_SHA").map(|s| s.get(..8).unwrap_or(s)).unwrap_or(tl!("development build"));
    ui.label(crate::i18n::fmt(tl!("Version {version} ({build})"), &[("version", version()), ("build", build)]));
    ui.label(text("A distraction-free word processor for writers, written in Rust."));
    ui.label(text("Based on WordCraft, by the ArtCraft team and the WordCraft contributors."));
    ui.add_space(6.0);
    ui.hyperlink_to(text("WriteCraft source code: github.com/ymauray/writecraft"), "https://github.com/ymauray/writecraft");
    ui.hyperlink_to(text("WordCraft source code: github.com/storytold/wordcraft"), "https://github.com/storytold/wordcraft");
    ui.add_space(6.0);
    ui.label(
        egui::RichText::new(text("MIT OR Apache-2.0. Copyright (c) 2026 ArtCraft Team, the WordCraft contributors and the WriteCraft contributors."))
            .small()
            .weak(),
    );
}

/// WriteCraft's own tabs: Pomodoro, before Help.
pub(crate) fn add_tabs(tabs: &mut Vec<(String, bool)>) {
    if !trimmed() || tabs.iter().any(|(n, _)| n == "Pomodoro") {
        return;
    }
    let at = tabs.iter().position(|(n, _)| n == "Help").unwrap_or(tabs.len());
    tabs.insert(at, ("Pomodoro".to_string(), false));
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

/// Whether the group titled `title` loses its dialog launcher (Styles: no styles pane; Font:
/// no Font dialog).
pub(crate) fn hides_launcher(title: &str) -> bool {
    trimmed() && matches!(title, "Styles" | "Font")
}

/// The Home groups left off the ribbon. Font stays, with Bold, Italic and Underline only.
pub const HIDDEN_HOME_GROUPS: [&str; 3] = ["Paragraph", "Voice", "Editor"];

/// Whether the Home tab leaves out the group titled `title`.
pub(crate) fn hides_home_group(title: &str) -> bool {
    trimmed() && HIDDEN_HOME_GROUPS.contains(&title)
}

/// The Home KeyTips that stay with the trims: those of the groups still shown.
pub(crate) const HOME_KEYTIPS: [&str; 10] = ["Paste", "Cut", "Copy", "Format Painter", "Bold", "Italic", "Underline", "Find", "Replace", "Select"];

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
    fn home_keeps_bold_italic_underline_and_leaves_out_the_rest() {
        set_trimmed(true);
        let mut h = home_harness();
        h.run_steps(4);
        for gone in [
            (Role::TextInput, "Font"),
            (Role::TextInput, "Font Size"),
            (Role::Button, "Strikethrough"),
            (Role::Button, "Font Color"),
            (Role::Button, "Align Left"),
            (Role::Button, "Dictate"),
        ] {
            assert!(h.query_by_role_and_label(gone.0, tl!(gone.1)).is_none(), "{} is gone", gone.1);
        }
        for kept in ["Bold", "Italic", "Underline"] {
            assert!(h.query_by_role_and_label(Role::Button, tl!(kept)).is_some(), "{kept} stays");
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
        for tab in ["File", "Home", "Review", "View", "Pomodoro", "Help"] {
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
    fn the_about_dialog_names_writecraft() {
        set_trimmed(true);
        let mut h = home_harness();
        h.state_mut().run("ui.dialog", serde_json::json!({"name": "about"})).unwrap();
        h.run_steps(4);
        assert!(h.query_by_label("WriteCraft").is_some());
        assert!(h.query_by_label(text("WriteCraft source code: github.com/ymauray/writecraft")).is_some());
        assert!(h.query_by_label(tl!("Join us on Discord: discord.gg/artcraft")).is_none());
        assert_eq!(dialog_title("About WordCraft"), "About WriteCraft");
        assert_eq!(dialog_title("Word Count"), tl!("Word Count"));
        assert_eq!(dialog_title("WordCraft"), "WriteCraft");
        assert_eq!(app_name(), "WriteCraft");
        set_trimmed(false);
        assert_eq!(dialog_title("About WordCraft"), tl!("About WordCraft"));
        assert_eq!(dialog_title("WordCraft"), tl!("WordCraft"));
        assert_eq!(app_name(), "WordCraft");
        // Every WriteCraft text has a French version.
        assert!(TEXTS.iter().all(|(en, fr)| !fr.is_empty() && en != fr));
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
