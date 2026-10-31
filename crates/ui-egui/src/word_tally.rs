//! WriteCraft's writing counters: the words written today and in the current pomodoro, shown in
//! the status bar beside the document's word count.
//!
//! Both are net counts: each frame the active document's word count is compared with the last one
//! seen for that document, and the difference (new words, or words deleted) is added. A document
//! seen for the first time (just opened or created) only sets its starting point, so opening a
//! long file doesn't count as writing it. Today's count is saved (`words-today.json`) so it
//! survives a restart, and starts again from zero on a new day.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::WordApp;
use crate::writecraft::text;

/// Which document a count belongs to: its id in the process, its file (opening a file into a
/// window gives it one) and whether notes are counted (a preference that changes the count).
type DocKey = (u64, Option<PathBuf>, bool);

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct Saved {
    /// The local date, `YYYY-MM-DD`.
    day: String,
    words: i64,
}

#[derive(Debug, Default)]
pub struct Tally {
    day: String,
    today: i64,
    /// The pomodoro the session count belongs to, and its count.
    session_of: Option<u32>,
    session: i64,
    last: HashMap<DocKey, usize>,
}

impl Tally {
    /// Takes in the active document's count. `pomodoro` is the timer's work session and whether
    /// words count for it now. Returns whether today's count changed.
    fn observe(&mut self, key: DocKey, words: usize, day: &str, pomodoro: Option<(u32, bool)>) -> bool {
        let mut changed = false;
        if self.day != day {
            self.day = day.to_string();
            self.today = 0;
            changed = true;
        }
        if let Some((n, _)) = pomodoro
            && self.session_of != Some(n)
        {
            self.session_of = Some(n);
            self.session = 0;
        }
        let words_now = i64::try_from(words).unwrap_or(i64::MAX);
        let before = self.last.insert(key, words);
        if let Some(before) = before {
            let delta = words_now.saturating_sub(i64::try_from(before).unwrap_or(i64::MAX));
            if delta != 0 {
                self.today = self.today.saturating_add(delta);
                if pomodoro.is_some_and(|(_, counting)| counting) {
                    self.session = self.session.saturating_add(delta);
                }
                changed = true;
            }
        }
        changed
    }

    fn state(&self) -> Value {
        json!({ "day": self.day, "today": self.today, "session": self.session_of.map(|_| self.session) })
    }
}

thread_local! {
    static TALLY: RefCell<Tally> = RefCell::new(Tally::default());
    static FILE: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
    /// When today's count was last saved, and whether it changed since.
    static SAVE: RefCell<(f64, bool)> = const { RefCell::new((f64::NEG_INFINITY, false)) };
}

fn with<R>(f: impl FnOnce(&mut Tally) -> R) -> R {
    TALLY.with(|t| f(&mut t.borrow_mut()))
}

/// The local date, `YYYY-MM-DD`.
fn local_day() -> String {
    #[cfg(not(target_arch = "wasm32"))]
    {
        chrono::Local::now().format("%Y-%m-%d").to_string()
    }
    #[cfg(target_arch = "wasm32")]
    {
        let d = js_sys::Date::new_0();
        format!("{:04}-{:02}-{:02}", d.get_full_year(), d.get_month() + 1, d.get_date())
    }
}

/// Today's count is read from and saved to `path` (the desktop app's settings folder). A count
/// from another day is ignored.
pub fn use_file(path: PathBuf) {
    if let Ok(bytes) = std::fs::read(&path)
        && let Ok(saved) = serde_json::from_slice::<Saved>(&bytes)
        && saved.day == local_day()
    {
        with(|t| {
            t.day = saved.day;
            t.today = saved.words;
        });
    }
    FILE.with(|f| *f.borrow_mut() = Some(path));
}

fn save() {
    let Some(path) = FILE.with(|f| f.borrow().clone()) else { return };
    let saved = with(|t| Saved { day: t.day.clone(), words: t.today });
    match serde_json::to_vec_pretty(&saved) {
        Ok(bytes) => {
            if let Err(e) = std::fs::write(&path, bytes) {
                log::warn!("couldn't save today's word count to {}: {e}", path.display());
            }
        }
        Err(e) => log::warn!("couldn't write today's word count: {e}"),
    }
}

/// Per frame: counts what changed in the active document, and saves today's count now and then.
pub fn tick(app: &mut WordApp) {
    if !app.has_document() {
        return;
    }
    let key = (app.session.document_id(), app.session.path.clone(), app.session.prefs.count_notes);
    let words = app.cached_word_count();
    let pomodoro = crate::pomodoro::work_session();
    let changed = with(|t| t.observe(key, words, &local_day(), pomodoro));
    let now = crate::clock::now_ms();
    SAVE.with(|s| {
        let mut s = s.borrow_mut();
        s.1 |= changed;
        // At most every two seconds: a save per keystroke would be wasteful.
        if s.1 && now - s.0 >= 2000.0 {
            *s = (now, false);
            save();
        }
    });
}

/// Runs a `ui.wordTally.*` command: `state` (`today`: words written today; `session`: in the
/// current or last pomodoro, null before the first) or `resetToday`.
pub fn command(id: &str) -> Result<Value, String> {
    match id {
        "ui.wordTally.state" => {}
        "ui.wordTally.resetToday" => {
            with(|t| t.today = 0);
            save();
        }
        other => return Err(format!("unknown command `{other}`")),
    }
    Ok(with(|t| t.state()))
}

/// A signed count: `+12`, `-3`, `0`.
fn signed(n: i64) -> String {
    if n > 0 { format!("+{n}") } else { n.to_string() }
}

/// The status bar's texts: today's words, and the pomodoro's once one has started.
pub(crate) fn status_texts() -> (String, Option<String>) {
    with(|t| {
        let today = text("Today: {n}").replace("{n}", &signed(t.today));
        let session = t.session_of.map(|_| text("Session: {n}").replace("{n}", &signed(t.session)));
        (today, session)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(id: u64) -> DocKey {
        (id, None, false)
    }

    #[test]
    fn counts_net_words_per_document_from_when_each_is_first_seen() {
        let mut t = Tally::default();
        assert!(t.observe(key(1), 5000, "2026-10-31", None), "a new day");
        assert_eq!(t.today, 0, "opening a document isn't writing it");
        t.observe(key(1), 5120, "2026-10-31", None);
        t.observe(key(1), 5100, "2026-10-31", None);
        assert_eq!(t.today, 100);
        // Another document: its own starting point.
        assert!(!t.observe(key(2), 300, "2026-10-31", None));
        t.observe(key(2), 350, "2026-10-31", None);
        t.observe(key(1), 5110, "2026-10-31", None);
        assert_eq!(t.today, 160);
        // The same window with a file opened into it starts again too.
        t.observe((2, Some(PathBuf::from("/a.docx")), false), 9000, "2026-10-31", None);
        assert_eq!(t.today, 160);
        // Midnight.
        t.observe(key(1), 5115, "2026-11-01", None);
        assert_eq!(t.today, 5);
    }

    #[test]
    fn the_session_counts_only_while_a_pomodoro_runs_and_restarts_with_the_next() {
        let mut t = Tally::default();
        let day = "2026-10-31";
        t.observe(key(1), 100, day, None);
        t.observe(key(1), 110, day, None);
        assert_eq!(t.state()["session"], Value::Null, "no pomodoro yet");
        t.observe(key(1), 150, day, Some((1, true)));
        assert_eq!((t.today, t.session), (50, 40));
        t.observe(key(1), 170, day, Some((1, false)));
        assert_eq!((t.today, t.session), (70, 40), "a break or a pause doesn't count");
        t.observe(key(1), 175, day, Some((2, true)));
        assert_eq!((t.today, t.session), (75, 5), "a new pomodoro starts from zero");
    }

    #[test]
    fn texts_and_commands() {
        with(|t| *t = Tally { session_of: Some(1), session: -3, today: 12, ..Tally::default() });
        assert_eq!(status_texts(), ("Today: +12".to_string(), Some("Session: -3".to_string())));
        assert_eq!(command("ui.wordTally.state").unwrap()["today"], 12);
        assert_eq!(command("ui.wordTally.resetToday").unwrap()["today"], 0);
        assert!(command("ui.wordTally.nope").is_err());
        with(|t| *t = Tally::default());
    }

    #[test]
    fn todays_count_is_read_back_only_on_the_same_day() {
        let dir = std::env::temp_dir().join(format!("writecraft-tally-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("words-today.json");
        std::fs::write(&path, serde_json::to_vec(&Saved { day: local_day(), words: 42 }).unwrap()).unwrap();
        use_file(path.clone());
        assert_eq!(with(|t| t.today), 42);
        with(|t| *t = Tally::default());
        std::fs::write(&path, serde_json::to_vec(&Saved { day: "1999-01-01".into(), words: 42 }).unwrap()).unwrap();
        use_file(path.clone());
        assert_eq!(with(|t| t.today), 0);
        std::fs::write(&path, b"junk").unwrap();
        use_file(path);
        assert_eq!(with(|t| t.today), 0);
        FILE.with(|f| *f.borrow_mut() = None);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
