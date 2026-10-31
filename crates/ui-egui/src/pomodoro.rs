//! WriteCraft's Pomodoro timer: work periods and short or long breaks, with the usual settings
//! (lengths, a long break every N pomodoros, auto-starting the next period, a notice at the end).
//!
//! The Pomodoro tab drives it, the status bar shows it (right of the document's language), and
//! the `ui.pomodoro.*` commands run it from scripts and agents like any button. The timer lives
//! on the UI thread; its arithmetic takes the time as a parameter, so tests set the clock.

use std::cell::RefCell;
use std::path::PathBuf;

use egui::Ui;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::WordApp;
use crate::writecraft::text;

pub mod sound;
use sound::{Cue, Theme};

/// The longest period a setting accepts, in minutes.
pub const MAX_MINUTES: u32 = 180;
/// The most pomodoros before a long break.
pub const MAX_LONG_EVERY: u32 = 12;

/// A period of the cycle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Phase {
    Work,
    ShortBreak,
    LongBreak,
}

impl Phase {
    fn id(self) -> &'static str {
        match self {
            Phase::Work => "work",
            Phase::ShortBreak => "shortBreak",
            Phase::LongBreak => "longBreak",
        }
    }
}

/// The timer's settings, saved between runs.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub work_minutes: u32,
    pub short_break_minutes: u32,
    pub long_break_minutes: u32,
    /// A long break after this many pomodoros.
    pub long_break_every: u32,
    pub auto_start_breaks: bool,
    pub auto_start_pomodoros: bool,
    /// Ask for attention (the Dock icon bounces) and say so on the status line when a period ends.
    pub notify: bool,
    /// The sounds at the start of a pomodoro or a break and at the end of a period.
    pub sounds: Theme,
    /// Their volume, 0–100 %.
    pub volume: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            work_minutes: 25,
            short_break_minutes: 5,
            long_break_minutes: 15,
            long_break_every: 4,
            auto_start_breaks: false,
            auto_start_pomodoros: false,
            notify: true,
            sounds: Theme::Beeps,
            volume: 70,
        }
    }
}

impl Settings {
    /// The settings with every number in range (a hand-edited file can hold anything).
    fn clamped(mut self) -> Self {
        self.work_minutes = self.work_minutes.clamp(1, MAX_MINUTES);
        self.short_break_minutes = self.short_break_minutes.clamp(1, MAX_MINUTES);
        self.long_break_minutes = self.long_break_minutes.clamp(1, MAX_MINUTES);
        self.long_break_every = self.long_break_every.clamp(1, MAX_LONG_EVERY);
        self.volume = self.volume.min(100);
        self
    }

    fn minutes(&self, phase: Phase) -> u32 {
        match phase {
            Phase::Work => self.work_minutes,
            Phase::ShortBreak => self.short_break_minutes,
            Phase::LongBreak => self.long_break_minutes,
        }
    }

    fn total_ms(&self, phase: Phase) -> f64 {
        f64::from(self.minutes(phase)) * 60_000.0
    }
}

/// The timer. Times are milliseconds on the UI's monotonic clock.
#[derive(Clone, Debug, PartialEq)]
pub struct Timer {
    pub settings: Settings,
    pub phase: Phase,
    /// Pomodoros finished since the cycle started.
    pub completed: u32,
    pub running: bool,
    /// Time left while paused.
    remaining_ms: f64,
    /// When the period ends while running.
    deadline_ms: f64,
    /// Started at least once since the last stop.
    pub started: bool,
    /// Sounds to play for what just happened (taken by `take_cues`).
    cues: Vec<Cue>,
    /// Counts the pomodoros started from the top, so the session word count knows a new one began.
    pub work_sessions: u32,
}

impl Default for Timer {
    fn default() -> Self {
        Timer::new(Settings::default())
    }
}

impl Timer {
    pub fn new(settings: Settings) -> Self {
        let settings = settings.clamped();
        let remaining_ms = settings.total_ms(Phase::Work);
        Timer { settings, phase: Phase::Work, completed: 0, running: false, remaining_ms, deadline_ms: 0.0, started: false, cues: Vec::new(), work_sessions: 0 }
    }

    pub fn total_ms(&self) -> f64 {
        self.settings.total_ms(self.phase)
    }

    pub fn remaining_ms(&self, now: f64) -> f64 {
        let left = if self.running { self.deadline_ms - now } else { self.remaining_ms };
        if left.is_finite() { left.clamp(0.0, self.total_ms()) } else { 0.0 }
    }

    /// The pomodoro being worked on or just finished, counting from 1.
    pub fn number(&self) -> u32 {
        if self.phase == Phase::Work { self.completed.saturating_add(1) } else { self.completed.max(1) }
    }

    /// Its place in the cycle before the long break, counting from 1.
    pub fn place_in_cycle(&self) -> u32 {
        (self.number().saturating_sub(1) % self.settings.long_break_every) + 1
    }

    pub fn start(&mut self, now: f64) {
        if !self.running {
            // A period starting from the top (not resumed after a pause) is announced.
            if self.remaining_ms >= self.total_ms() {
                self.cues.push(if self.phase == Phase::Work { Cue::WorkStart } else { Cue::BreakStart });
                if self.phase == Phase::Work {
                    self.work_sessions = self.work_sessions.wrapping_add(1);
                }
            }
            self.deadline_ms = now + self.remaining_ms;
            self.running = true;
            self.started = true;
        }
    }

    pub fn pause(&mut self, now: f64) {
        if self.running {
            self.remaining_ms = self.remaining_ms(now);
            self.running = false;
        }
    }

    pub fn toggle(&mut self, now: f64) {
        if self.running { self.pause(now) } else { self.start(now) }
    }

    /// Restart the current period from its full length.
    pub fn reset(&mut self, now: f64) {
        self.remaining_ms = self.total_ms();
        self.deadline_ms = now + self.remaining_ms;
    }

    /// Back to the first pomodoro, stopped.
    pub fn stop(&mut self) {
        let sessions = self.work_sessions;
        *self = Timer { work_sessions: sessions, ..Timer::new(self.settings.clone()) };
    }

    /// Go to the next period. A skipped pomodoro isn't counted as finished.
    pub fn skip(&mut self, now: f64) {
        let next = match self.phase {
            Phase::Work if self.completed.saturating_add(1).is_multiple_of(self.settings.long_break_every) => Phase::LongBreak,
            Phase::Work => Phase::ShortBreak,
            _ => Phase::Work,
        };
        self.enter(next, now, self.running);
    }

    /// Ends the period when its time is up; returns the period that ended.
    pub fn tick(&mut self, now: f64) -> Option<Phase> {
        if !self.running || self.remaining_ms(now) > 0.0 {
            return None;
        }
        let ended = self.phase;
        self.cues.push(Cue::End);
        let next = if ended == Phase::Work {
            self.completed = self.completed.saturating_add(1);
            if self.completed.is_multiple_of(self.settings.long_break_every) { Phase::LongBreak } else { Phase::ShortBreak }
        } else {
            Phase::Work
        };
        let auto = if next == Phase::Work { self.settings.auto_start_pomodoros } else { self.settings.auto_start_breaks };
        self.enter(next, now, auto);
        Some(ended)
    }

    fn enter(&mut self, phase: Phase, now: f64, running: bool) {
        self.phase = phase;
        self.remaining_ms = self.total_ms();
        self.running = false;
        if running {
            self.start(now);
        }
    }

    /// The sounds due since the last call, in order.
    pub fn take_cues(&mut self) -> Vec<Cue> {
        std::mem::take(&mut self.cues)
    }

    /// New settings. A period that hasn't begun takes its new length.
    pub fn set_settings(&mut self, settings: Settings) {
        let untouched = !self.running && self.remaining_ms >= self.total_ms();
        self.settings = settings.clamped();
        if untouched || self.remaining_ms > self.total_ms() {
            self.remaining_ms = self.total_ms();
        }
    }

    pub fn state(&self, now: f64) -> Value {
        json!({
            "phase": self.phase.id(),
            "running": self.running,
            "started": self.started,
            "number": self.number(),
            "placeInCycle": self.place_in_cycle(),
            "completed": self.completed,
            "remainingSeconds": (self.remaining_ms(now) / 1000.0).ceil(),
            "totalSeconds": self.total_ms() / 1000.0,
            "settings": self.settings,
        })
    }
}

thread_local! {
    static TIMER: RefCell<Timer> = RefCell::new(Timer::default());
    static SETTINGS_FILE: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
}

/// The pomodoro the session word count belongs to: its number (changes when a new one starts)
/// and whether words written now count (a pomodoro is running). `None` before the first one.
pub fn work_session() -> Option<(u32, bool)> {
    with(|t| (t.work_sessions > 0).then(|| (t.work_sessions, t.running && t.phase == Phase::Work)))
}

fn now() -> f64 {
    crate::clock::now_ms()
}

fn with<R>(f: impl FnOnce(&mut Timer) -> R) -> R {
    TIMER.with(|t| f(&mut t.borrow_mut()))
}

/// The settings are read from and saved to `path` (the desktop app's settings folder).
pub fn use_settings_file(path: PathBuf) {
    if let Ok(bytes) = std::fs::read(&path)
        && let Ok(settings) = serde_json::from_slice::<Settings>(&bytes)
    {
        with(|t| t.set_settings(settings));
    }
    SETTINGS_FILE.with(|f| *f.borrow_mut() = Some(path));
}

fn save_settings(settings: &Settings) {
    let Some(path) = SETTINGS_FILE.with(|f| f.borrow().clone()) else { return };
    match serde_json::to_vec_pretty(settings) {
        Ok(bytes) => {
            if let Err(e) = std::fs::write(&path, bytes) {
                log::warn!("couldn't save the Pomodoro settings to {}: {e}", path.display());
            }
        }
        Err(e) => log::warn!("couldn't write the Pomodoro settings: {e}"),
    }
}

/// A whole number of minutes (or pomodoros) in `1..=max`, if `key` is given.
fn count(p: &Value, key: &str, max: u32) -> Result<Option<u32>, String> {
    let Some(v) = p.get(key) else { return Ok(None) };
    let n = v.as_f64().filter(|n| n.is_finite() && n.fract() == 0.0 && *n >= 1.0 && *n <= f64::from(max));
    match n {
        // In range and whole, so the conversion is exact.
        Some(n) => Ok(u32::try_from(n as i64).ok()),
        None => Err(format!("{key} must be a whole number from 1 to {max}")),
    }
}

fn flag(p: &Value, key: &str) -> Result<Option<bool>, String> {
    match p.get(key) {
        None => Ok(None),
        Some(v) => v.as_bool().map(Some).ok_or_else(|| format!("{key} must be true or false")),
    }
}

fn set(t: &mut Timer, p: &Value) -> Result<(), String> {
    let mut s = t.settings.clone();
    if let Some(n) = count(p, "workMinutes", MAX_MINUTES)? {
        s.work_minutes = n;
    }
    if let Some(n) = count(p, "shortBreakMinutes", MAX_MINUTES)? {
        s.short_break_minutes = n;
    }
    if let Some(n) = count(p, "longBreakMinutes", MAX_MINUTES)? {
        s.long_break_minutes = n;
    }
    if let Some(n) = count(p, "longBreakEvery", MAX_LONG_EVERY)? {
        s.long_break_every = n;
    }
    if let Some(b) = flag(p, "autoStartBreaks")? {
        s.auto_start_breaks = b;
    }
    if let Some(b) = flag(p, "autoStartPomodoros")? {
        s.auto_start_pomodoros = b;
    }
    if let Some(b) = flag(p, "notify")? {
        s.notify = b;
    }
    if let Some(v) = p.get("sounds") {
        s.sounds = v.as_str().and_then(Theme::from_id).ok_or("sounds must be \"off\", \"beeps\" or \"fun\"")?;
    }
    if let Some(v) = p.get("volume") {
        let n = v.as_f64().filter(|n| n.is_finite() && n.fract() == 0.0 && (0.0..=100.0).contains(n));
        s.volume = n.map(|n| n as u32).ok_or("volume must be a whole number from 0 to 100")?;
    }
    t.set_settings(s);
    save_settings(&t.settings);
    Ok(())
}

/// Runs a `ui.pomodoro.*` command: `start`, `pause`, `toggle`, `reset` (restart the period),
/// `skip` (next period), `stop` (back to the first pomodoro), `set` (settings: `workMinutes`,
/// `shortBreakMinutes`, `longBreakMinutes`, `longBreakEvery`, `autoStartBreaks`,
/// `autoStartPomodoros`, `notify`, `sounds`: `off`/`beeps`/`fun`, `volume`: 0–100), `preview`
/// (play the sounds: `cue` = `workStart`, `breakStart` or `end`, or all three) and `state`. Each
/// returns the timer's state.
pub fn command(id: &str, p: &Value) -> Result<Value, String> {
    let now = now();
    let (state, cues, settings) = with(|t| {
        match id.strip_prefix("ui.pomodoro.").unwrap_or("") {
            "start" => t.start(now),
            "pause" => t.pause(now),
            "toggle" => t.toggle(now),
            "reset" => t.reset(now),
            "skip" => t.skip(now),
            "stop" => t.stop(),
            "set" => set(t, p)?,
            "preview" => {
                let cue = match p.get("cue") {
                    None => None,
                    Some(v) => Some(v.as_str().unwrap_or("")),
                };
                let cues: &[Cue] = match cue {
                    None => &[Cue::WorkStart, Cue::BreakStart, Cue::End],
                    Some("workStart") => &[Cue::WorkStart],
                    Some("breakStart") => &[Cue::BreakStart],
                    Some("end") => &[Cue::End],
                    Some(_) => return Err("cue must be \"workStart\", \"breakStart\" or \"end\"".into()),
                };
                t.cues.extend_from_slice(cues);
            }
            "state" => {}
            other => return Err(format!("unknown command `ui.pomodoro.{other}`")),
        }
        Ok::<_, String>((t.state(now), t.take_cues(), t.settings.clone()))
    })?;
    sound::play(settings.sounds, settings.volume, &cues);
    Ok(state)
}

/// Per frame: ends a period whose time is up, says so, and keeps the display moving.
pub fn tick(app: &mut WordApp, ctx: &egui::Context) {
    let now = now();
    let (ended, running, notify, next, cues, settings) = with(|t| {
        let ended = t.tick(now);
        (ended, t.running, t.settings.notify, t.phase, t.take_cues(), t.settings.clone())
    });
    sound::play(settings.sounds, settings.volume, &cues);
    if let Some(ended) = ended
        && notify
    {
        ctx.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(egui::UserAttentionType::Informational));
        let msg = match (ended, next) {
            (Phase::Work, Phase::LongBreak) => text("Pomodoro finished: time for a long break."),
            (Phase::Work, _) => text("Pomodoro finished: time for a break."),
            _ => text("Break over: back to work."),
        };
        app.status(msg.to_string());
    }
    if running {
        ctx.request_repaint_after(std::time::Duration::from_millis(250));
    }
}

fn clock(ms: f64) -> String {
    // Whole seconds, rounded up so the display reaches 00:00 as the period ends.
    let secs = (ms / 1000.0).ceil().clamp(0.0, f64::from(MAX_MINUTES) * 60.0) as u32;
    format!("{:02}:{:02}", secs / 60, secs % 60)
}

/// The status bar's text and its tooltip.
pub(crate) fn status_text() -> (String, &'static str) {
    let now = now();
    let shown = with(|t| {
        let times = format!("{} / {}", clock(t.remaining_ms(now)), clock(t.total_ms()));
        let mut s = match t.phase {
            Phase::Work => format!("Pomodoro {} ({}/{}) · {times}", t.number(), t.place_in_cycle(), t.settings.long_break_every),
            Phase::ShortBreak => format!("{} · {times}", text("Short break")),
            Phase::LongBreak => format!("{} · {times}", text("Long break")),
        };
        if t.started && !t.running {
            s.push_str(" · ");
            s.push_str(text("Paused"));
        }
        s
    });
    (shown, text("Click to start or pause the timer"))
}

const MINUTES: [u32; 12] = [1, 5, 10, 15, 20, 25, 30, 35, 40, 45, 50, 60];

/// A labelled minutes box; returns the chosen whole number.
fn number_box(ui: &mut Ui, id: &str, label: &str, label_w: f32, value: u32, items: &[u32]) -> Option<u32> {
    let mut out = None;
    crate::widgets::row(ui, |ui| {
        let (r, _) = ui.allocate_exact_size(egui::vec2(label_w, 20.0), egui::Sense::hover());
        let t = crate::theme::Tokens::get(ui.ctx());
        ui.painter().text(egui::pos2(r.min.x, r.center().y), egui::Align2::LEFT_CENTER, label, crate::theme::regular(12.0), t.text);
        let items: Vec<String> = items.iter().map(u32::to_string).collect();
        if let Some(v) = crate::widgets::combo_hint(ui, id, 52.0, &value.to_string(), &items, None, label) {
            out = v.trim().parse::<u32>().ok();
        }
    });
    out
}

/// The Pomodoro tab.
pub(crate) fn ribbon(app: &mut WordApp, ui: &mut Ui) {
    let (running, started, settings) = with(|t| (t.running, t.started, t.settings.clone()));
    crate::widgets::group_titled(ui, "Pomodoro timer", text("Timer"), None, app, |ui, app| {
        let (icon, label) = if running {
            ("mediaPause", text("Pause"))
        } else if started {
            ("mediaPlay", text("Resume"))
        } else {
            ("mediaPlay", text("Start"))
        };
        crate::widgets::big(ui, app, icon, label, "ui.pomodoro.toggle", json!({}), false);
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(1.0, 1.0);
            let b = |id: &'static str| crate::widgets::ButtonAction::new(id, json!({}), false);
            crate::widgets::small(ui, app, "replay", Some(text("Reset")), text("Restart the current period"), b("ui.pomodoro.reset"));
            crate::widgets::small(ui, app, "mediaNext", Some(text("Skip")), text("Go to the next period"), b("ui.pomodoro.skip"));
            crate::widgets::small(ui, app, "mediaStop", Some(text("Stop")), text("Stop and reset the cycle"), b("ui.pomodoro.stop"));
        });
    });
    crate::widgets::group_titled(ui, "Pomodoro durations", text("Durations"), None, app, |ui, app| {
        let boxes: [(&str, &str, u32, &[u32]); 4] = [
            ("workMinutes", "Pomodoro (min)", settings.work_minutes, &MINUTES),
            ("shortBreakMinutes", "Short break (min)", settings.short_break_minutes, &MINUTES),
            ("longBreakMinutes", "Long break (min)", settings.long_break_minutes, &MINUTES),
            ("longBreakEvery", "Long break after (pomodoros)", settings.long_break_every, &[2, 3, 4, 5, 6, 8]),
        ];
        // Two columns of two, so the boxes fit the ribbon's height.
        for column in boxes.chunks(2) {
            // The column's labels share the widest one's width, so its boxes line up.
            let label_w = column
                .iter()
                .map(|(_, label, ..)| {
                    ui.ctx().fonts_mut(|f| f.layout_no_wrap(text(label).to_string(), crate::theme::regular(12.0), egui::Color32::BLACK).size().x)
                })
                .fold(0.0_f32, f32::max)
                + 8.0;
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(1.0, 4.0);
                for (key, label, value, items) in column.iter().copied() {
                    if let Some(n) = number_box(ui, key, text(label), label_w, value, items)
                        && let Err(e) = app.run("ui.pomodoro.set", json!({ key: n }))
                    {
                        app.status(e);
                    }
                }
            });
            ui.add_space(8.0);
        }
    });
    crate::widgets::group_titled(ui, "Pomodoro options", text("Options"), None, app, |ui, app| {
        ui.vertical(|ui| {
            let checks = [
                ("autoStartBreaks", "Auto-start breaks", settings.auto_start_breaks),
                ("autoStartPomodoros", "Auto-start pomodoros", settings.auto_start_pomodoros),
                ("notify", "Notify at the end", settings.notify),
            ];
            for (key, label, mut on) in checks {
                if ui.checkbox(&mut on, text(label)).changed() {
                    let _ = app.run("ui.pomodoro.set", json!({ key: on }));
                }
            }
        });
    });
    crate::widgets::group_titled(ui, "Pomodoro sounds", text("Sounds"), None, app, |ui, app| {
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(1.0, 4.0);
            let labels = [text("Sounds"), text("Volume (%)")];
            let label_w = labels
                .iter()
                .map(|l| ui.ctx().fonts_mut(|f| f.layout_no_wrap(l.to_string(), crate::theme::regular(12.0), egui::Color32::BLACK).size().x))
                .fold(0.0_f32, f32::max)
                + 8.0;
            crate::widgets::row(ui, |ui| {
                let (r, _) = ui.allocate_exact_size(egui::vec2(label_w, 20.0), egui::Sense::hover());
                let t = crate::theme::Tokens::get(ui.ctx());
                ui.painter().text(egui::pos2(r.min.x, r.center().y), egui::Align2::LEFT_CENTER, labels[0], crate::theme::regular(12.0), t.text);
                let names: Vec<String> = Theme::ALL.iter().map(|t| text(t.label()).to_string()).collect();
                let current = text(settings.sounds.label());
                if let Some(v) = crate::widgets::combo_hint(ui, "pomodoroSounds", 90.0, current, &names, None, labels[0])
                    && let Some(theme) = Theme::ALL.into_iter().find(|t| text(t.label()) == v.trim())
                {
                    let _ = app.run("ui.pomodoro.set", json!({ "sounds": theme.id() }));
                    let _ = app.run("ui.pomodoro.preview", json!({ "cue": "workStart" }));
                }
            });
            if let Some(n) = number_box(ui, "volume", labels[1], label_w, settings.volume, &[10, 25, 40, 55, 70, 85, 100])
                && let Err(e) = app.run("ui.pomodoro.set", json!({ "volume": n.min(100) }))
            {
                app.status(e);
            }
            let b = crate::widgets::ButtonAction::new("ui.pomodoro.preview", json!({}), false);
            crate::widgets::small(ui, app, "readAloud", Some(text("Listen")), text("Play the start, break and end sounds"), b);
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIN: f64 = 60_000.0;

    #[test]
    fn a_cycle_runs_work_and_breaks_with_a_long_break_every_fourth() {
        let mut t = Timer::default();
        assert_eq!((t.phase, t.number(), t.remaining_ms(0.0)), (Phase::Work, 1, 25.0 * MIN));
        t.start(0.0);
        assert_eq!(t.remaining_ms(10.0 * MIN), 15.0 * MIN);
        assert_eq!(t.tick(24.0 * MIN), None);
        assert_eq!(t.tick(25.0 * MIN), Some(Phase::Work));
        // A break waits for a start unless breaks start by themselves.
        assert_eq!((t.phase, t.running, t.completed, t.number()), (Phase::ShortBreak, false, 1, 1));
        let mut now = 25.0 * MIN;
        for n in 2..=4 {
            t.start(now);
            now += 5.0 * MIN;
            assert_eq!(t.tick(now), Some(Phase::ShortBreak));
            assert_eq!((t.phase, t.number(), t.place_in_cycle()), (Phase::Work, n, n));
            t.start(now);
            now += 25.0 * MIN;
            assert_eq!(t.tick(now), Some(Phase::Work));
        }
        assert_eq!((t.phase, t.completed, t.remaining_ms(now)), (Phase::LongBreak, 4, 15.0 * MIN));
        t.start(now);
        assert_eq!(t.tick(now + 15.0 * MIN), Some(Phase::LongBreak));
        assert_eq!((t.phase, t.number(), t.place_in_cycle()), (Phase::Work, 5, 1));
    }

    #[test]
    fn pause_resume_reset_skip_and_stop() {
        let mut t = Timer::default();
        t.start(0.0);
        t.pause(5.0 * MIN);
        assert_eq!(t.remaining_ms(50.0 * MIN), 20.0 * MIN, "a paused timer keeps its time");
        t.toggle(50.0 * MIN);
        assert_eq!(t.remaining_ms(52.0 * MIN), 18.0 * MIN);
        t.reset(52.0 * MIN);
        assert_eq!(t.remaining_ms(52.0 * MIN), 25.0 * MIN);
        t.skip(52.0 * MIN);
        assert_eq!((t.phase, t.completed, t.running), (Phase::ShortBreak, 0, true), "a skipped pomodoro doesn't count");
        t.skip(53.0 * MIN);
        assert_eq!(t.phase, Phase::Work);
        t.stop();
        assert_eq!(t.work_sessions, 2, "the first pomodoro and the one skipped back to");
        assert_eq!(t, Timer { work_sessions: 2, ..Timer::default() }, "stop keeps only the session count");
    }

    #[test]
    fn starts_and_ends_queue_their_sounds_but_resuming_does_not() {
        let mut t = Timer::new(Settings { auto_start_breaks: true, ..Settings::default() });
        t.start(0.0);
        assert_eq!(t.take_cues(), [Cue::WorkStart]);
        t.pause(MIN);
        t.start(2.0 * MIN);
        assert!(t.take_cues().is_empty(), "a resume is silent");
        assert_eq!(t.tick(26.0 * MIN), Some(Phase::Work));
        assert_eq!(t.take_cues(), [Cue::End, Cue::BreakStart], "the end, then the break that started by itself");
        assert_eq!(t.tick(31.0 * MIN), Some(Phase::ShortBreak));
        assert_eq!(t.take_cues(), [Cue::End], "the next pomodoro waits for a start");
        assert!(command("ui.pomodoro.preview", &json!({"cue": "end"})).is_ok());
        assert!(command("ui.pomodoro.preview", &json!({"cue": 3})).is_err());
        for bad in [json!({"sounds": "loud"}), json!({"sounds": 1}), json!({"volume": 101}), json!({"volume": -1}), json!({"volume": 0.5})] {
            assert!(command("ui.pomodoro.set", &bad).is_err(), "{bad}");
        }
        assert_eq!(command("ui.pomodoro.set", &json!({"sounds": "fun", "volume": 0})).unwrap()["settings"]["sounds"], "fun");
        let _ = command("ui.pomodoro.set", &json!({"sounds": "beeps", "volume": 70}));
    }

    #[test]
    fn auto_start_settings_chain_the_periods() {
        let mut t = Timer::new(Settings { auto_start_breaks: true, auto_start_pomodoros: true, ..Settings::default() });
        t.start(0.0);
        assert_eq!(t.tick(25.0 * MIN), Some(Phase::Work));
        assert!(t.running && t.phase == Phase::ShortBreak);
        assert_eq!(t.tick(30.0 * MIN), Some(Phase::ShortBreak));
        assert!(t.running && t.phase == Phase::Work);
    }

    #[test]
    fn settings_apply_to_a_period_not_yet_begun_and_are_clamped() {
        let mut t = Timer::default();
        t.set_settings(Settings { work_minutes: 50, ..Settings::default() });
        assert_eq!(t.remaining_ms(0.0), 50.0 * MIN);
        t.set_settings(Settings { work_minutes: 0, long_break_every: 999, ..Settings::default() });
        assert_eq!((t.settings.work_minutes, t.settings.long_break_every), (1, MAX_LONG_EVERY));
    }

    #[test]
    fn commands_validate_their_parameters_and_never_panic() {
        assert!(command("ui.pomodoro.set", &json!({"workMinutes": 30})).is_ok());
        for bad in [
            json!({"workMinutes": 0}),
            json!({"workMinutes": 181}),
            json!({"workMinutes": 2.5}),
            json!({"workMinutes": "25"}),
            json!({"workMinutes": f64::MAX}),
            json!({"longBreakEvery": -3}),
            json!({"notify": 1}),
        ] {
            assert!(command("ui.pomodoro.set", &bad).is_err(), "{bad}");
        }
        assert!(command("ui.pomodoro.nope", &json!({})).is_err());
        for id in ["start", "pause", "toggle", "reset", "skip", "stop", "state"] {
            let v = command(&format!("ui.pomodoro.{id}"), &json!([1, {"x": null}])).unwrap();
            assert!(v["remainingSeconds"].as_f64().is_some(), "{id}");
        }
        assert_eq!(command("ui.pomodoro.state", &json!({})).unwrap()["settings"]["workMinutes"], 30);
        let _ = command("ui.pomodoro.set", &json!({"workMinutes": 25}));
        let _ = command("ui.pomodoro.stop", &json!({}));
    }

    #[test]
    fn the_status_text_shows_the_period_and_its_times() {
        let _ = command("ui.pomodoro.stop", &json!({}));
        let (s, _) = status_text();
        assert_eq!(s, "Pomodoro 1 (1/4) · 25:00 / 25:00");
        let _ = command("ui.pomodoro.start", &json!({}));
        let _ = command("ui.pomodoro.pause", &json!({}));
        assert!(status_text().0.ends_with(" · Paused"), "{}", status_text().0);
        let _ = command("ui.pomodoro.skip", &json!({}));
        assert!(status_text().0.starts_with("Short break · 05:00 / 05:00"), "{}", status_text().0);
        let _ = command("ui.pomodoro.stop", &json!({}));
        assert_eq!(clock(f64::NAN), "00:00");
        assert_eq!(clock(61_001.0), "01:02");
    }

    #[test]
    fn saved_settings_are_read_back_and_bad_files_ignored() {
        let dir = std::env::temp_dir().join(format!("writecraft-pomodoro-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("pomodoro.json");
        std::fs::write(&path, b"not json").unwrap();
        use_settings_file(path.clone());
        assert_eq!(command("ui.pomodoro.state", &json!({})).unwrap()["settings"]["workMinutes"], 25);
        command("ui.pomodoro.set", &json!({"workMinutes": 40, "notify": false})).unwrap();
        with(|t| *t = Timer::default());
        use_settings_file(path.clone());
        let s = command("ui.pomodoro.state", &json!({})).unwrap();
        assert_eq!((s["settings"]["workMinutes"].as_u64(), s["settings"]["notify"].as_bool()), (Some(40), Some(false)));
        SETTINGS_FILE.with(|f| *f.borrow_mut() = None);
        with(|t| *t = Timer::default());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
