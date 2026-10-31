//! The Pomodoro timer's sounds. Every sound is synthesised here from a few notes (no sound files,
//! so nothing to attribute) and played on a short-lived thread, so the interface never waits.

use serde::{Deserialize, Serialize};

/// Samples per second of the synthesised sounds.
pub const RATE: u32 = 44_100;
/// The longest sequence a single play renders, in seconds (a guard on the buffer's size).
const MAX_SECONDS: f32 = 6.0;

/// Which set of sounds plays.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Theme {
    Off,
    /// Short electronic beeps.
    #[default]
    Beeps,
    /// Little tunes: a fanfare, a slide whistle, a cuckoo.
    Fun,
}

impl Theme {
    pub const ALL: [Theme; 3] = [Theme::Off, Theme::Beeps, Theme::Fun];

    pub fn id(self) -> &'static str {
        match self {
            Theme::Off => "off",
            Theme::Beeps => "beeps",
            Theme::Fun => "fun",
        }
    }

    pub fn from_id(id: &str) -> Option<Theme> {
        Theme::ALL.into_iter().find(|t| t.id() == id)
    }

    /// The English name shown in the ribbon (translated through `writecraft::text`).
    pub fn label(self) -> &'static str {
        match self {
            Theme::Off => "No sound",
            Theme::Beeps => "Beeps",
            Theme::Fun => "Fun",
        }
    }
}

/// A moment the timer marks with a sound.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cue {
    /// A pomodoro begins.
    WorkStart,
    /// A break begins.
    BreakStart,
    /// A period's time is up.
    End,
}

#[derive(Clone, Copy)]
enum Wave {
    Sine,
    Triangle,
    /// A sine with two soft overtones and a long ring.
    Bell,
}

/// One note: starts at `at` seconds, lasts `len`, glides from `from` to `to` Hz.
#[derive(Clone, Copy)]
struct Note {
    at: f32,
    len: f32,
    from: f32,
    to: f32,
    wave: Wave,
    gain: f32,
    /// Vibrato depth as a fraction of the pitch (6 Hz wobble).
    vibrato: f32,
}

const fn note(at: f32, len: f32, hz: f32, wave: Wave) -> Note {
    Note { at, len, from: hz, to: hz, wave, gain: 1.0, vibrato: 0.0 }
}

// Pitches (Hz).
const C5: f32 = 523.25;
const E5: f32 = 659.25;
const G5: f32 = 783.99;
const C6: f32 = 1046.5;

fn notes(theme: Theme, cue: Cue) -> Vec<Note> {
    use Wave::*;
    match (theme, cue) {
        (Theme::Off, _) => Vec::new(),
        // Two beeps going up: off we go.
        (Theme::Beeps, Cue::WorkStart) => vec![note(0.0, 0.11, 880.0, Sine), note(0.16, 0.18, 1320.0, Sine)],
        // Two beeps going down: ease off.
        (Theme::Beeps, Cue::BreakStart) => vec![note(0.0, 0.11, 1320.0, Sine), note(0.16, 0.18, 880.0, Sine)],
        // Three quick beeps, like a kitchen timer.
        (Theme::Beeps, Cue::End) => (0..3).map(|i| note(i as f32 * 0.17, 0.1, 1000.0, Sine)).collect(),
        // A little fanfare: do-mi-sol-DO.
        (Theme::Fun, Cue::WorkStart) => vec![
            note(0.0, 0.09, C5, Triangle),
            note(0.1, 0.09, E5, Triangle),
            note(0.2, 0.09, G5, Triangle),
            Note { vibrato: 0.012, ..note(0.3, 0.45, C6, Triangle) },
        ],
        // A slide whistle sliding down, then a soft "plop".
        (Theme::Fun, Cue::BreakStart) => vec![
            Note { from: 1500.0, to: 380.0, vibrato: 0.02, ..note(0.0, 0.75, 0.0, Sine) },
            Note { from: 300.0, to: 120.0, gain: 0.8, ..note(0.8, 0.09, 0.0, Sine) },
        ],
        // A cuckoo clock, twice.
        (Theme::Fun, Cue::End) => [0.0, 0.7]
            .into_iter()
            .flat_map(|t| [Note { gain: 0.9, ..note(t, 0.22, E5, Bell) }, Note { gain: 0.9, ..note(t + 0.26, 0.32, C5, Bell) }])
            .collect(),
    }
}

/// Renders `cues` one after another, mono, peaking at `volume` (0–100 %).
pub fn render(theme: Theme, volume: u32, cues: &[Cue]) -> Vec<f32> {
    let mut all: Vec<Note> = Vec::new();
    let mut offset = 0.0_f32;
    for &cue in cues {
        let ns = notes(theme, cue);
        let end = ns.iter().map(|n| n.at + n.len + release(n.wave)).fold(0.0_f32, f32::max);
        all.extend(ns.into_iter().map(|n| Note { at: n.at + offset, ..n }));
        if end > 0.0 {
            offset += end + 0.15;
        }
    }
    let seconds = all.iter().map(|n| n.at + n.len + release(n.wave)).fold(0.0_f32, f32::max).min(MAX_SECONDS);
    if seconds <= 0.0 || volume == 0 {
        return Vec::new();
    }
    let len = (seconds * RATE as f32).ceil() as usize + 1;
    let mut out = vec![0.0_f32; len];
    for n in &all {
        add_note(&mut out, n);
    }
    let peak = out.iter().fold(0.0_f32, |m, s| m.max(s.abs()));
    if peak > 0.0 {
        let scale = 0.6 * volume.min(100) as f32 / 100.0 / peak;
        out.iter_mut().for_each(|s| *s *= scale);
    }
    out
}

fn release(wave: Wave) -> f32 {
    match wave {
        Wave::Bell => 0.35,
        _ => 0.03,
    }
}

fn add_note(out: &mut [f32], n: &Note) {
    let tau = std::f32::consts::TAU;
    let rate = RATE as f32;
    let start = (n.at * rate) as usize;
    let total = n.len + release(n.wave);
    let count = (total * rate) as usize;
    let mut phase = 0.0_f32;
    for i in 0..count {
        let t = i as f32 / rate;
        let Some(slot) = out.get_mut(start.saturating_add(i)) else { break };
        let glide = (t / n.len).min(1.0);
        let wobble = 1.0 + n.vibrato * (tau * 6.0 * t).sin();
        let hz = (n.from + (n.to - n.from) * glide) * wobble;
        phase = (phase + tau * hz / rate) % tau;
        let v = match n.wave {
            Wave::Sine => phase.sin(),
            Wave::Triangle => 2.0 / std::f32::consts::PI * phase.sin().asin(),
            Wave::Bell => phase.sin() + 0.4 * (2.0 * phase).sin() + 0.15 * (3.0 * phase).sin(),
        };
        // A 5 ms attack, then hold (or ring down, for a bell) and a short release.
        let attack = (t / 0.005).min(1.0);
        let body = match n.wave {
            Wave::Bell => (-t * 5.0).exp(),
            _ if t > n.len => (1.0 - (t - n.len) / release(n.wave)).max(0.0),
            _ => 1.0,
        };
        *slot += v * attack * body * n.gain;
    }
}

/// Plays `cues` in the background; does nothing when the theme is off or no output device opens.
pub fn play(theme: Theme, volume: u32, cues: &[Cue]) {
    if theme == Theme::Off || cues.is_empty() || volume == 0 {
        return;
    }
    let samples = render(theme, volume, cues);
    if samples.is_empty() {
        return;
    }
    output(samples);
}

#[cfg(all(not(test), not(target_arch = "wasm32")))]
fn output(samples: Vec<f32>) {
    use std::num::NonZero;
    let (Some(channels), Some(rate)) = (NonZero::new(1_u16), NonZero::new(RATE)) else { return };
    let seconds = samples.len() as f32 / RATE as f32;
    let spawned = std::thread::Builder::new().name("pomodoro-sound".into()).spawn(move || {
        let mut sink = match open_sink() {
            Ok(sink) => sink,
            Err(e) => {
                log::warn!("no sound output for the Pomodoro timer: {e} ({e:?})");
                return;
            }
        };
        sink.log_on_drop(false);
        sink.mixer().add(rodio::buffer::SamplesBuffer::new(channels, rate, samples));
        // Keep the device open until the sound has played out.
        std::thread::sleep(std::time::Duration::from_secs_f32(seconds + 0.2));
    });
    if let Err(e) = spawned {
        log::warn!("couldn't play the Pomodoro sound: {e}");
    }
}

/// The default output device. rodio asks for ~50 ms of buffer, which some devices (Bluetooth
/// headphones: at most 1024 frames) refuse, so smaller buffers are tried first.
#[cfg(all(not(test), not(target_arch = "wasm32")))]
fn open_sink() -> Result<rodio::MixerDeviceSink, rodio::DeviceSinkError> {
    for frames in [1024, 512] {
        let sink = rodio::DeviceSinkBuilder::from_default_device()
            .and_then(|b| b.with_buffer_size(rodio::cpal::BufferSize::Fixed(frames)).open_stream());
        if sink.is_ok() {
            return sink;
        }
    }
    rodio::DeviceSinkBuilder::open_default_sink()
}

/// Tests and the web build stay silent.
#[cfg(any(test, target_arch = "wasm32"))]
fn output(_samples: Vec<f32>) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_theme_and_cue_renders_a_bounded_sound() {
        for theme in [Theme::Beeps, Theme::Fun] {
            for cue in [Cue::WorkStart, Cue::BreakStart, Cue::End] {
                let s = render(theme, 100, &[cue]);
                assert!(!s.is_empty() && s.len() < (MAX_SECONDS * RATE as f32) as usize + 2, "{theme:?} {cue:?}");
                let peak = s.iter().fold(0.0_f32, |m, v| m.max(v.abs()));
                assert!(s.iter().all(|v| v.is_finite()) && (peak - 0.6).abs() < 1e-3, "{theme:?} {cue:?} peaks at {peak}");
            }
        }
        assert!(render(Theme::Off, 100, &[Cue::End]).is_empty());
        assert!(render(Theme::Fun, 0, &[Cue::End]).is_empty());
        // A sequence plays its cues one after another.
        let one = render(Theme::Beeps, 50, &[Cue::End]).len();
        let two = render(Theme::Beeps, 50, &[Cue::End, Cue::BreakStart]).len();
        assert!(two > one);
        let peak = render(Theme::Beeps, 50, &[Cue::End]).iter().fold(0.0_f32, |m, v| m.max(v.abs()));
        assert!((peak - 0.3).abs() < 1e-3);
    }

    #[test]
    fn theme_ids_round_trip() {
        for t in Theme::ALL {
            assert_eq!(Theme::from_id(t.id()), Some(t));
        }
        assert_eq!(Theme::from_id("loud"), None);
    }
}
