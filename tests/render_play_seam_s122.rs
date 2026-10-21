//! tests/render_play_seam_s122.rs — S122 Phase 0: the `render` / `play` seam (`src/render.rs`).
//!
//! Integration tests over the PUBLIC API of `audiohax::render`, the one orchestration seam every
//! front-end (CLI, TUI, the planned GUI) now calls. They pin four properties that the CLI
//! byte-identity harness (one process per render, `audiohax render --wav`) cannot see:
//!
//!   (a) S40 one-builder. What `play` dispatches to an `AudioSink` IS the timeline `render`
//!       synthesizes, stated exactly: `play` sends the eager ProgramChange init (one per voice,
//!       voice order, `programs[i]` or the legacy floor) and THEN every timeline command in
//!       timeline order. The timeline's own first `instrument_count` events are those same
//!       ProgramChanges at `at_ms = 0`, so a full stream sends them twice. (`main.rs@1d5c4db`
//!       L1183–1195 did the same; the eager init is "kept for the MIDI-port ready handshake".)
//!   (b) BUG-01, library half. A pre-set cancel and a mid-stream cancel both return `Ok`
//!       promptly and dispatch no further TIMELINE events. The eager init does not poll the
//!       token, so a cancel set before or during it still lets all `instrument_count` eager
//!       ProgramChanges through (HEAD behaviour, pinned by `EAGER_PC_INIT_IGNORES_CANCEL`).
//!   (c) Same-thread register hygiene. The seed, expression and cadence registers are
//!       `thread_local!`; a GUI worker renders repeatedly on one thread. A render after a
//!       differently-configured render on the same thread must equal the same render on a fresh
//!       thread. The event dump is process-global; whether it stays armed after an error raised
//!       between `arm` and `disarm` is pinned too.
//!   (d) In-repo identity guard. `render()` + the lib's own `write_stereo_wav` produce the SAME
//!       WAV bytes the shipped binary (`CARGO_BIN_EXE_audiohax render --wav`) writes, and the
//!       numbers the CLI prints equal `RenderOutput.summary`. Same machine, same build. This
//!       guards the thin CLI adapter (arguments, WAV writer, printed numbers). It is NOT a
//!       regression guard for the sound: since S122 the CLI calls `render()`, so a change
//!       INSIDE `render()` moves both sides equally and passes here (QG mutation M8,
//!       `s122-06-phase0-seal.md` § 6). The guard against a change in the rendered audio is
//!       `tests/render_golden_s122.rs`, which compares against pre-S122 recorded values.
//!
//! Two register/dump leaks found in S122 TE wave 2 are pinned by `KNOWN_*` constants. Both were
//! FIXED in implementer round 2 and both constants are now `false`, so (c3) and (c4) assert the
//! FIXED behaviour: a returning leak turns them red. Fix the code; do NOT flip a constant back to
//! `true` to silence it, and never loosen the assertion.
//!
//! Fixture: `assets/images/Lena.png`, the only committed PNG, at `--steps 6` (the scan only;
//! the piece's length comes from the plan). Every comparison here is same-machine and
//! same-build, so decoder portability (s121-01 §7 U1) does not apply. Debug-build cost is about
//! 1 s of analysis and 4–5 s of synthesis per render, so the fresh-thread reference results are
//! computed ONCE per test binary (`REF_*`, each on its own new thread) and shared.
//!
//! Filesystem: only test (d) writes files (two WAVs in `std::env::temp_dir()`, removed after),
//! because the property under test IS the bytes the binary writes to disk. Everything else is in
//! memory.
//!
//! Concurrency: the event dump is process-global. Test (c4) arms it and reads `is_armed()`, so it
//! takes `DUMP` exclusively; every other test takes it shared and may run in parallel (none of
//! them arms or disarms the dump).
#![cfg(all(feature = "synth", feature = "pure-analysis"))]

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::time::{Duration, Instant};

use audiohax::cli::{PipelineArgs, Toggle, V2Spec};
use audiohax::engine::{AudioSink, AudioSinkError};
use audiohax::mapping_loader::{load_mappings, MappingTable};
use audiohax::render::{
    play, prepare_play, render, CancelToken, ImageInput, PlayEvent, PlayPlan, RenderError,
    RenderNote, RenderOutput, RenderRequest,
};
use audiohax::synth_sink::{write_stereo_wav, MidiCmd, SoundFontSource, TimedMidiEvent};

// ─────────────────────────────────────────────────────────────────────────────
// Pinned behaviours (flip the constant, never loosen the assertion)
// ─────────────────────────────────────────────────────────────────────────────

/// `play()` sends one ProgramChange per voice BEFORE the stream and BEFORE its first
/// `cancel.is_cancelled()` read (render.rs `play`, moved from main.rs@1d5c4db L1183–1195). So a
/// pre-set cancel still sends `instrument_count` ProgramChanges, and a cancel raised during the
/// init does not cut it short. If the lead rules the init must honour the token, flip this.
const EAGER_PC_INIT_IGNORES_CANCEL: bool = true;

/// FIXED defect (found S122 TE wave 2; fixed in implementer round 2). The public
/// `*_with_source(.., understanding: None, ..)` entries (the OpenCV arm) did not reset the
/// thread-local cadence register, so a `None` render after a pure render on the same thread
/// inherited the previous image's cadence figure. `compose()` now calls
/// `clear_cadence_development()` on the `None` arm. `false` = fixed (current tree): (c3) asserts
/// the register is `neutral()` and the plan equals a fresh thread's. `true` would assert the leak.
const KNOWN_LEAK_CADENCE_ON_NONE_ARM: bool = false;

/// FIXED defect (implementer's U-6, pre-existing at HEAD; fixed in implementer round 2 by the
/// `DumpArmGuard` drop guard). An error raised between `expression_dump::arm` and `disarm`
/// (here: the caller's note handler failing on `ExpressionVerified`) used to leave the
/// process-global dump ARMED, so the next render in the process recorded into a stale dump.
/// `false` = disarmed after the error (current tree): (c4) asserts it. `true` would assert the leak.
const KNOWN_LEAK_DUMP_ARMED_AFTER_ERROR: bool = false;

// ─────────────────────────────────────────────────────────────────────────────
// Fixtures
// ─────────────────────────────────────────────────────────────────────────────

static DUMP: RwLock<()> = RwLock::new(());

fn shared() -> RwLockReadGuard<'static, ()> {
    DUMP.read().unwrap_or_else(|e| e.into_inner())
}

fn exclusive() -> RwLockWriteGuard<'static, ()> {
    DUMP.write().unwrap_or_else(|e| e.into_inner())
}

/// Scan steps for every request here (and the CLI run in (d)). See the module doc.
const STEPS: usize = 6;

fn manifest(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(rel)
}

fn lena() -> PathBuf {
    manifest("assets/images/Lena.png")
}

fn mappings() -> MappingTable {
    load_mappings(
        manifest("assets/mappings.json")
            .to_str()
            .expect("utf-8 path"),
    )
    .expect("assets/mappings.json loads")
}

fn request(seed: Option<u64>, expression: bool, v2: Option<&str>) -> RenderRequest {
    RenderRequest {
        seed,
        loop_cue: false,
        expression: if expression { Toggle::On } else { Toggle::Off },
        v2: v2.map(|s| s.parse::<V2Spec>().expect("v2 spec parses")),
        pipeline: PipelineArgs {
            steps: STEPS,
            ..PipelineArgs::default()
        },
        reverb: Toggle::On,
        gain: 1.0,
        event_dump: None,
    }
}

fn plain() -> RenderRequest {
    request(Some(7), false, None)
}

/// Expression on, every v2 term, same seed as `plain()`.
fn armed() -> RenderRequest {
    request(Some(7), true, Some("all"))
}

fn quiet() -> impl FnMut(RenderNote<'_>) -> Result<(), RenderError> {
    |_| Ok(())
}

fn render_lena(req: &RenderRequest, m: &MappingTable) -> RenderOutput {
    let p = lena();
    render(
        req,
        ImageInput::Path(&p),
        m,
        SoundFontSource::Bundled,
        &mut quiet(),
    )
    .expect("render(Lena)")
}

fn prepare_lena(req: &RenderRequest, m: &MappingTable) -> PlayPlan {
    let p = lena();
    prepare_play(req, ImageInput::Path(&p), m, &mut quiet()).expect("prepare_play(Lena)")
}

fn bits(v: &[f32]) -> Vec<u32> {
    v.iter().map(|s| s.to_bits()).collect()
}

/// Run `f` on a brand-new thread, i.e. with every thread-local register at its initializer.
fn on_fresh_thread<T: Send>(f: impl FnOnce() -> T + Send) -> T {
    std::thread::scope(|s| s.spawn(f).join().expect("fresh thread"))
}

/// Fresh-thread reference results, computed once per test binary, each on its own new thread.
fn ref_plain_render() -> &'static RenderOutput {
    static R: OnceLock<RenderOutput> = OnceLock::new();
    R.get_or_init(|| on_fresh_thread(|| render_lena(&plain(), &mappings())))
}
fn ref_plain_plan() -> &'static PlayPlan {
    static R: OnceLock<PlayPlan> = OnceLock::new();
    R.get_or_init(|| on_fresh_thread(|| prepare_lena(&plain(), &mappings())))
}
fn ref_armed_render() -> &'static RenderOutput {
    static R: OnceLock<RenderOutput> = OnceLock::new();
    R.get_or_init(|| on_fresh_thread(|| render_lena(&armed(), &mappings())))
}
fn ref_armed_plan() -> &'static PlayPlan {
    static R: OnceLock<PlayPlan> = OnceLock::new();
    R.get_or_init(|| on_fresh_thread(|| prepare_lena(&armed(), &mappings())))
}

/// Records every command `play` dispatches, in order. It can raise a `CancelToken` immediately
/// AFTER dispatch number `cancel_after` (1-based), which makes "mid-stream" exact and free of
/// wall-clock jitter.
struct RecordingSink {
    cmds: Arc<Mutex<Vec<MidiCmd>>>,
    cancel_after: Option<(usize, CancelToken)>,
}

impl RecordingSink {
    fn new() -> Self {
        RecordingSink {
            cmds: Arc::new(Mutex::new(Vec::new())),
            cancel_after: None,
        }
    }
    fn push(&mut self, c: MidiCmd) {
        let n = {
            let mut v = self.cmds.lock().unwrap();
            v.push(c);
            v.len()
        };
        if let Some((k, tok)) = &self.cancel_after {
            if n == *k {
                tok.cancel();
            }
        }
    }
    fn taken(&self) -> Vec<MidiCmd> {
        self.cmds.lock().unwrap().clone()
    }
}

impl AudioSink for RecordingSink {
    fn note_on(&mut self, channel: u8, note: u8, velocity: u8) -> Result<(), AudioSinkError> {
        self.push(MidiCmd::NoteOn {
            channel,
            note,
            velocity,
        });
        Ok(())
    }
    fn note_off(&mut self, channel: u8, note: u8) -> Result<(), AudioSinkError> {
        self.push(MidiCmd::NoteOff { channel, note });
        Ok(())
    }
    fn program_change(&mut self, channel: u8, program: u8) -> Result<(), AudioSinkError> {
        self.push(MidiCmd::ProgramChange { channel, program });
        Ok(())
    }
}

/// The eager init `play` sends, computed from the plan's own fields (NOT from the timeline).
fn eager_init(plan: &PlayPlan) -> Vec<MidiCmd> {
    (0..plan.instrument_count)
        .map(|i| MidiCmd::ProgramChange {
            channel: (i % 16) as u8,
            program: plan
                .programs
                .get(i)
                .copied()
                .unwrap_or_else(|| audiohax::instrumentation::legacy_program(i)),
        })
        .collect()
}

/// Exactly what a full, uncancelled `play(plan)` must dispatch.
fn expected_dispatch(plan: &PlayPlan) -> Vec<MidiCmd> {
    let mut v = eager_init(plan);
    v.extend(plan.timeline.iter().map(|e| e.cmd));
    v
}

/// `plan` with every `at_ms` set to 0 and the ORDER kept. `play` never reorders; `at_ms` only
/// decides how long it sleeps and which `Step` it reports. Streaming this copy therefore checks
/// the whole dispatch sequence with no wall-clock wait (the API takes no clock).
fn clock_collapsed(plan: &PlayPlan) -> PlayPlan {
    let mut p = plan.clone();
    for e in &mut p.timeline {
        e.at_ms = 0;
    }
    p
}

fn tag(ev: &PlayEvent) -> String {
    match ev {
        PlayEvent::Started { .. } => "Started".into(),
        PlayEvent::Step(i) => format!("Step({i})"),
        PlayEvent::Cancelled => "Cancelled".into(),
        PlayEvent::SinkError(e) => format!("SinkError({e:?})"),
        PlayEvent::Finished => "Finished".into(),
    }
}

/// Stream `plan` into `sink`; returns (play's own elapsed time, progress tags). Asserts `Ok`.
fn stream(
    plan: &PlayPlan,
    sink: &mut RecordingSink,
    cancel: &CancelToken,
) -> (Duration, Vec<String>) {
    let mut progress = Vec::new();
    let t = Instant::now();
    let res = play(plan, sink, cancel, &mut |ev| progress.push(tag(&ev)));
    let dt = t.elapsed();
    assert!(res.is_ok(), "play() must return Ok, got {:?}", res.err());
    (dt, progress)
}

// ─────────────────────────────────────────────────────────────────────────────
// (a) S40 one-builder
// ─────────────────────────────────────────────────────────────────────────────

/// (a) `render().events` == `prepare_play().timeline`, and `play` dispatches exactly
/// `eager_init ++ timeline` — whose first `instrument_count` commands are the timeline's own
/// t=0 ProgramChanges, i.e. the init is a duplicate of the timeline head. Checked with the
/// expression axis off AND fully armed (`--expression on --v2 all`).
#[test]
fn s40_play_dispatches_eager_init_then_exactly_the_render_timeline() {
    let _g = shared();
    for (out, plan) in [
        (ref_plain_render(), ref_plain_plan()),
        (ref_armed_render(), ref_armed_plan()),
    ] {
        assert_eq!(
            out.events, plan.timeline,
            "render and play must share ONE timeline (S40)"
        );
        assert_eq!(out.summary.total_steps, plan.total_steps);
        assert_eq!(out.summary.grid_ms_per_step, plan.grid_ms_per_step);
        assert_eq!(out.summary.instrument_count, plan.instrument_count);
        assert_eq!(out.summary.programs, plan.programs);

        let n = plan.instrument_count;
        assert!(
            n > 0 && plan.timeline.len() > n,
            "plan too small to be a real piece"
        );
        let head: Vec<MidiCmd> = plan.timeline[..n].iter().map(|e| e.cmd).collect();
        assert!(plan.timeline[..n].iter().all(|e| e.at_ms == 0));
        assert_eq!(
            eager_init(plan),
            head,
            "the eager init must equal the timeline's own t=0 ProgramChange head"
        );

        let collapsed = clock_collapsed(plan);
        let mut sink = RecordingSink::new();
        let (_dt, progress) = stream(&collapsed, &mut sink, &CancelToken::default());
        assert_eq!(
            sink.taken(),
            expected_dispatch(plan),
            "play must dispatch eager_init ++ timeline, command for command"
        );
        assert_eq!(progress, vec!["Started", "Step(0)", "Finished"]);
    }
    assert_ne!(
        ref_plain_plan().timeline,
        ref_armed_plan().timeline,
        "control: the armed request must compose a different timeline"
    );
}

/// (a), real time. Same property on the UNcollapsed plan, so the sleep path runs too, and the
/// `Step` sequence is checked against `at_ms / grid`. Streams the whole piece at wall-clock speed.
#[test]
#[ignore = "streams the whole piece in real time (tens of seconds); play() takes no clock — run with --ignored"]
fn s40_full_realtime_play_equals_render_timeline() {
    let _g = shared();
    let plan = ref_plain_plan();
    assert_eq!(ref_plain_render().events, plan.timeline);
    let mut sink = RecordingSink::new();
    let (dt, progress) = stream(plan, &mut sink, &CancelToken::default());
    assert_eq!(sink.taken(), expected_dispatch(plan));
    let last = plan.timeline.last().unwrap().at_ms;
    assert!(
        dt >= Duration::from_millis(last),
        "streamed faster than the plan ({dt:?} < {last} ms)"
    );
    let mut steps: Vec<String> = Vec::new();
    for e in &plan.timeline {
        let s = format!(
            "Step({})",
            ((e.at_ms / plan.grid_ms_per_step) as usize).min(plan.total_steps - 1)
        );
        if steps.last() != Some(&s) {
            steps.push(s);
        }
    }
    let mut expect = vec!["Started".to_string()];
    expect.extend(steps);
    expect.push("Finished".into());
    assert_eq!(progress, expect);
}

// ─────────────────────────────────────────────────────────────────────────────
// (b) BUG-01, library half
// ─────────────────────────────────────────────────────────────────────────────

/// (b) A cancel set BEFORE `play()`: `Ok` at once, NO timeline event dispatched (so no NoteOn),
/// progress `[Started, Cancelled, Finished]`. Only the eager init reaches the sink (H2).
#[test]
fn bug01_preset_cancel_returns_ok_promptly_and_dispatches_no_timeline_event() {
    let _g = shared();
    let plan = ref_plain_plan();
    let cancel = CancelToken::default();
    cancel.cancel();
    let mut sink = RecordingSink::new();
    let (dt, progress) = stream(plan, &mut sink, &cancel);
    let got = sink.taken();
    if EAGER_PC_INIT_IGNORES_CANCEL {
        assert_eq!(
            got,
            eager_init(plan),
            "a pre-set cancel sends exactly the eager init"
        );
    } else {
        assert!(
            got.is_empty(),
            "a pre-set cancel must dispatch nothing, got {got:?}"
        );
    }
    assert!(got
        .iter()
        .all(|c| matches!(c, MidiCmd::ProgramChange { .. })));
    assert_eq!(progress, vec!["Started", "Cancelled", "Finished"]);
    assert!(
        dt < Duration::from_millis(250),
        "pre-set cancel took {dt:?}"
    );
}

/// (b) A cancel raised mid-stream, right after dispatch `k`, stops at EXACTLY `k` (the next
/// top-of-event poll), returns `Ok`, and what was sent is a prefix of `eager_init ++ timeline`.
/// Three `k`: inside the eager init (it does not poll ⇒ all `n` go out, H2), inside the t=0
/// burst (no sleep), and just past the first non-zero onset (one real sleep).
#[test]
fn bug01_midstream_cancel_stops_at_exactly_the_cancelling_dispatch() {
    let _g = shared();
    let plan = ref_plain_plan();
    let n = plan.instrument_count;
    let exp = expected_dispatch(plan);
    let first_nonzero = plan
        .timeline
        .iter()
        .position(|e| e.at_ms > 0)
        .expect("piece has a second onset");
    assert!(
        first_nonzero > n + 1,
        "t=0 burst too short to cancel inside it"
    );
    let cases = [
        (1, "inside the eager init"),
        (n + first_nonzero - 1, "inside the t=0 burst"),
        (n + first_nonzero + 1, "past the first non-zero onset"),
    ];
    for (k, what) in cases {
        let cancel = CancelToken::default();
        let mut sink = RecordingSink::new();
        sink.cancel_after = Some((k, cancel.clone()));
        let (dt, progress) = stream(plan, &mut sink, &cancel);
        let got = sink.taken();
        let stop = if k <= n && EAGER_PC_INIT_IGNORES_CANCEL {
            n
        } else {
            k
        };
        assert_eq!(
            got.len(),
            stop,
            "cancel after dispatch {k} ({what}) must stop at {stop}"
        );
        assert_eq!(got[..], exp[..stop], "dispatch prefix diverges ({what})");
        assert_eq!(
            progress[progress.len() - 2..],
            ["Cancelled".to_string(), "Finished".to_string()],
            "({what}) {progress:?}"
        );
        // Bound: the plan time of the last event sent, plus generous slack for a loaded box.
        let plan_ms = if stop > n {
            plan.timeline[stop - n - 1].at_ms
        } else {
            0
        };
        assert!(
            dt < Duration::from_millis(plan_ms + 400),
            "({what}) play took {dt:?} for {plan_ms} ms of plan"
        );
    }
}

/// (b) A cancel that lands while `play` SLEEPS toward a future event is honoured within the
/// poll slice (20 ms), not at that event, and nothing further is sent. Pinned exactly as the
/// tree behaves: the step change is reported BEFORE the sleep (so `Step(1)` appears although no
/// step-1 event is sent), and the sleep loop breaks the stream with NO `Cancelled`
/// (main.rs@1d5c4db printed no "Shutdown requested" on that path either). Synthetic plan with a
/// 2 s gap: the property is plan-independent and this keeps the test ~0.2 s.
#[test]
fn bug01_cancel_during_sleep_is_honoured_within_a_poll_slice() {
    let _g = shared();
    let ev = |at_ms, cmd| TimedMidiEvent { at_ms, cmd };
    let plan = PlayPlan {
        timeline: vec![
            ev(
                0,
                MidiCmd::ProgramChange {
                    channel: 0,
                    program: 1,
                },
            ),
            ev(
                0,
                MidiCmd::NoteOn {
                    channel: 0,
                    note: 60,
                    velocity: 90,
                },
            ),
            ev(
                2_000,
                MidiCmd::NoteOff {
                    channel: 0,
                    note: 60,
                },
            ),
            ev(
                2_000,
                MidiCmd::NoteOn {
                    channel: 0,
                    note: 62,
                    velocity: 90,
                },
            ),
        ],
        total_steps: 2,
        grid_ms_per_step: 2_000,
        instrument_count: 1,
        programs: vec![1],
        source_step_count: 2,
    };
    let sent_before_gap = plan.instrument_count + 2;
    let cancel = CancelToken::default();
    let mut sink = RecordingSink::new();
    let cmds = Arc::clone(&sink.cmds);
    let c2 = cancel.clone();
    let watcher = std::thread::spawn(move || {
        let t0 = Instant::now();
        while cmds.lock().unwrap().len() < sent_before_gap {
            assert!(
                t0.elapsed() < Duration::from_secs(5),
                "t=0 events never arrived"
            );
            std::thread::sleep(Duration::from_millis(1));
        }
        std::thread::sleep(Duration::from_millis(100));
        c2.cancel();
        Instant::now()
    });
    let (_dt, progress) = stream(&plan, &mut sink, &cancel);
    let returned = Instant::now();
    let cancelled_at = watcher.join().expect("watcher");
    let lag = returned.saturating_duration_since(cancelled_at);
    assert!(
        lag < Duration::from_millis(150),
        "cancel during a sleep must be honoured within one 20 ms poll slice; took {lag:?}"
    );
    assert_eq!(
        sink.taken().len(),
        sent_before_gap,
        "nothing may be sent after the cancel"
    );
    assert_eq!(progress, vec!["Started", "Step(0)", "Step(1)", "Finished"]);
}

// ─────────────────────────────────────────────────────────────────────────────
// (c) Same-thread register hygiene
// ─────────────────────────────────────────────────────────────────────────────

/// (c1) Expression ON (+ every v2 term), then OFF, on one thread ⇒ the OFF render equals the
/// fresh-thread OFF render: events, samples (bit for bit) and summary; and the expression
/// register reads back `inert()` afterwards.
#[test]
fn registers_render_after_armed_render_equals_fresh_thread_render() {
    let _g = shared();
    let m = mappings();
    let armed_out = render_lena(&armed(), &m);
    let again = render_lena(&plain(), &m);
    let fresh = ref_plain_render();
    assert_eq!(
        armed_out.events,
        ref_armed_render().events,
        "armed render on this thread == fresh"
    );
    assert_ne!(
        armed_out.events, fresh.events,
        "control: the armed render must differ"
    );
    assert_eq!(
        again.events, fresh.events,
        "a register leaked into the timeline"
    );
    assert_eq!(
        bits(&again.interleaved),
        bits(&fresh.interleaved),
        "a register leaked into the samples"
    );
    assert_eq!(again.summary, fresh.summary);
    assert_eq!(
        audiohax::composition::expression_register::active(),
        audiohax::composition::ExpressionProfile::inert()
    );
}

/// (c2) The `play` twin (wave-1 U-a), in the GUI's order: an armed `render`, then
/// `prepare_play(OFF)` on the same thread, equals the fresh-thread `prepare_play(OFF)`. (The
/// armed step is `prepare_play`, not `render`: both set the registers in the one shared
/// `compose()`, and synthesis touches none; (c1) covers `render` → `render`.)
#[test]
fn registers_prepare_play_after_armed_prepare_equals_fresh_thread() {
    let _g = shared();
    let m = mappings();
    let armed_plan = prepare_lena(&armed(), &m);
    assert_eq!(&armed_plan, ref_armed_plan());
    let again = prepare_lena(&plain(), &m);
    assert_eq!(
        &again,
        ref_plain_plan(),
        "a register leaked into prepare_play"
    );
}

/// (c3) The `understanding: None` arm of the public `*_with_source` entries (the OpenCV entry,
/// reachable from any lib caller with a `FeatureSource`). A pure compose sets the cadence
/// register to Lena's figure; a following `None` compose on the same thread must reset it (it
/// did not before implementer round 2). Pinned by `KNOWN_LEAK_CADENCE_ON_NONE_ARM`: the register
/// value AND the timeline are compared against a fresh thread.
#[test]
fn registers_none_arm_cadence_after_a_pure_render() {
    use audiohax::chord_engine::{cadence_development, CadenceFigure};
    use audiohax::pure_analysis::{load_pure_image, PureAnalysisSource, PureImageSource};
    use audiohax::render::prepare_play_with_source;
    let _g = shared();
    let m = mappings();
    let req = plain();
    let none_plan = |m: &MappingTable| {
        let cfg = audiohax::cli::pipeline_to_engine_config(&req.pipeline);
        let img = load_pure_image(&PureImageSource::UserPath(lena())).expect("decode");
        let src = PureAnalysisSource::extract(
            &img,
            cfg.num_instruments,
            cfg.bar_thickness_frac,
            req.pipeline.steps,
            None,
        )
        .expect("extract");
        let plan = prepare_play_with_source(&req, &src, None, m, &mut quiet())
            .expect("prepare_play_with_source(None)");
        (plan, cadence_development())
    };
    let (fresh, fresh_reg) = on_fresh_thread(|| none_plan(&m));
    assert_eq!(fresh_reg, CadenceFigure::neutral(), "fresh thread: neutral");

    let _ = prepare_lena(&plain(), &m);
    let lena_reg = cadence_development();
    assert!(
        !lena_reg.is_neutral(),
        "control: a pure compose must set a live figure"
    );
    let (again, again_reg) = none_plan(&m);
    audiohax::chord_engine::clear_cadence_development();

    if KNOWN_LEAK_CADENCE_ON_NONE_ARM {
        assert_eq!(
            again_reg, lena_reg,
            "pinned leak: the None arm keeps the previous figure"
        );
        assert_ne!(
            again.timeline, fresh.timeline,
            "pinned leak: the leaked figure reaches the None-arm timeline"
        );
    } else {
        assert_eq!(
            again_reg,
            CadenceFigure::neutral(),
            "None arm must reset the figure"
        );
        assert_eq!(again, fresh);
    }
}

/// Disarms the global dump on drop, so a failing (c4) cannot leave it armed for another test.
struct DisarmOnDrop;
impl Drop for DisarmOnDrop {
    fn drop(&mut self) {
        audiohax::expression_dump::disarm();
    }
}

/// (c4) The event dump (process-global) when an error fires between `arm` and `disarm`: the
/// caller's note handler fails on `ExpressionVerified`. Pinned by
/// `KNOWN_LEAK_DUMP_ARMED_AFTER_ERROR` (now `false`: the dump must be disarmed after the error).
/// Also checked: the NEXT render in the process still equals a clean render (events, samples).
/// The label assertions run only when the constant is `true`; they record what the leak did
/// before round 2 (the stale dump recorded the unrelated render under the failed render's label).
#[test]
fn event_dump_after_an_error_between_arm_and_disarm() {
    use audiohax::expression_dump;
    use audiohax::render::DumpLabel;
    let clean = ref_plain_render(); // computed before taking the exclusive lock
    let _g = exclusive();
    let _cleanup = DisarmOnDrop;
    let m = mappings();
    assert!(!expression_dump::is_armed(), "precondition: dump disarmed");

    let mut req = request(Some(7), true, None);
    req.event_dump = Some(DumpLabel {
        cue: "s122-probe".into(),
        image: "lena".into(),
    });
    let p = lena();
    let mut failing = |n: RenderNote<'_>| match n {
        RenderNote::ExpressionVerified { .. } => {
            Err(RenderError::Refused("probe: handler failed".into()))
        }
        _ => Ok(()),
    };
    let err = render(
        &req,
        ImageInput::Path(&p),
        &m,
        SoundFontSource::Bundled,
        &mut failing,
    )
    .expect_err("the failing note handler must abort the render");
    assert_eq!(err.to_string(), "probe: handler failed");

    assert_eq!(
        expression_dump::is_armed(),
        KNOWN_LEAK_DUMP_ARMED_AFTER_ERROR,
        "dump armed-state after an error between arm and disarm"
    );

    let next = render_lena(&plain(), &m);
    assert_eq!(
        next.events, clean.events,
        "a stale dump must not move the timeline"
    );
    assert_eq!(
        bits(&next.interleaved),
        bits(&clean.interleaved),
        "a stale dump must not move the samples"
    );
    if KNOWN_LEAK_DUMP_ARMED_AFTER_ERROR {
        let snap = expression_dump::snapshot().expect("snapshot");
        assert_eq!(
            snap.cue, "s122-probe",
            "the next render records under the failed render's label"
        );
        assert!(
            !snap.events.is_empty(),
            "the stale dump recorded the unrelated render"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// (d) lib == CLI, same machine
// ─────────────────────────────────────────────────────────────────────────────

fn temp_wav(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "audiohax_s122_seam_{tag}_{}_{nanos}.wav",
        std::process::id()
    ))
}

/// (d) `render()` + `synth_sink::write_stereo_wav` writes the SAME bytes as
/// `audiohax render assets/images/Lena.png --seed 7 --steps 6 --wav …`, and the lines the CLI
/// prints carry `RenderOutput.summary`'s step/voice counts and the sample-derived duration
/// (wave-1 U-i). The binary runs from the manifest dir: it reads `assets/mappings.json`
/// cwd-relative.
#[test]
fn cli_render_wav_equals_lib_render_through_the_lib_wav_writer() {
    let _g = shared();
    let out = ref_plain_render();

    let lib_wav = temp_wav("lib");
    write_stereo_wav(&lib_wav, out.sample_rate, &out.interleaved).expect("lib writes WAV");
    let lib_bytes = std::fs::read(&lib_wav).expect("read lib WAV");
    let _ = std::fs::remove_file(&lib_wav);

    let cli_wav = temp_wav("cli");
    let run = std::process::Command::new(env!("CARGO_BIN_EXE_audiohax"))
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .args([
            "render",
            "assets/images/Lena.png",
            "--seed",
            "7",
            "--steps",
            &STEPS.to_string(),
            "--wav",
        ])
        .arg(&cli_wav)
        .output()
        .expect("spawn audiohax");
    let cli_bytes = std::fs::read(&cli_wav);
    let _ = std::fs::remove_file(&cli_wav);
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(
        run.status.success(),
        "CLI render failed: {}\n{stdout}\n{}",
        run.status,
        String::from_utf8_lossy(&run.stderr)
    );
    let cli_bytes = cli_bytes.expect("read CLI WAV");

    assert!(lib_bytes.len() > 44, "lib WAV has no samples");
    assert_eq!(lib_bytes.len(), cli_bytes.len(), "WAV length: lib vs CLI");
    assert!(
        lib_bytes == cli_bytes,
        "WAV bytes differ: lib render() vs the shipped binary"
    );

    let planned = format!(
        "render --wav: {} steps, {} instruments → {}",
        out.summary.total_steps,
        out.summary.instrument_count,
        cli_wav.display()
    );
    assert!(
        stdout.lines().any(|l| l == planned),
        "missing `{planned}` in:\n{stdout}"
    );
    let secs = out.interleaved.len() as f32 / 2.0 / out.sample_rate as f32;
    let wrote = format!(
        "render --wav: wrote {} ({secs:.1}s, font=bundled, reverb=on, gain=1)",
        cli_wav.display()
    );
    assert!(
        stdout.lines().any(|l| l == wrote),
        "missing `{wrote}` in:\n{stdout}"
    );
}
