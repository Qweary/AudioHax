//! The one worker thread (Shielding `s123-SH` § 4.2–§ 4.4; lead ruling L-3 carried).
//!
//! * **T-1.** Reading, checking and decoding the picture, `render()`, `prepare_play()` and
//!   `play()` all run here, never on the UI thread. The UI only sends [`JobKind`]s and reads
//!   [`Event`]s.
//! * **T-3.** There is ONE worker, and it runs one job at a time. While a job is running (an
//!   abandoned one included), a newly submitted job WAITS in a single pending slot (a newer
//!   submission replaces an older pending one); it never starts a second worker.
//! * **T-2.** There is no automatic timeout. "Cancel" means ABANDON: [`Controller::abandon`]
//!   forgets the job's generation, so its results are dropped when they arrive; the worker runs
//!   it to completion (Rust cannot stop the analysis safely). A playing job is also told to stop.
//! * **C-1.** Exactly one `std::panic::catch_unwind` wraps each whole job ([`run_guarded`]).
//! * **C-2.** The closure is `AssertUnwindSafe`. That holds because (a) the request, the picture
//!   bytes and the mapping table are only borrowed read-only (`compose` copies the table); (b) the
//!   only side effect a job has on the world outside it is SENDING messages on a channel — it
//!   mutates no GUI state; and (c) results reach the UI only as messages built from `Ok` values.
//! * **C-3.** [`install_panic_hook`] records the panic message and location (a double-clicked app
//!   has no console), and the caught panic is shown with that text under "Details".
//! * **W-1.** No job arms the event dump: every request is built by
//!   [`super::state::Controls::request`], whose `event_dump` is always `None`.
//! * **W-2.** After a caught panic the worker thread EXITS and the next job gets a fresh thread,
//!   so post-panic correctness does not rest on `compose()` resetting every thread-local register
//!   by convention.

use super::ingest::{decode_for_display, probe, read_bounded, DisplayImage, Probe, Refusal};
use super::sinks::{GuiMidiSink, MidiTarget, SilencingSink};
use super::text::{from_refusal, from_render_error, ErrorKind, UserError};
use crate::engine::AudioSink;
use crate::mapping_loader::MappingTable;
use crate::render::{
    play, prepare_play, render, CancelToken, ImageInput, PlayEvent, RenderOutput, RenderRequest,
};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Once};
use std::time::{Duration, Instant};

// ─────────────────────────────────────────────────────────────────────────────
// Panic hook (C-3)
// ─────────────────────────────────────────────────────────────────────────────

static HOOK: Once = Once::new();

thread_local! {
    /// The last panic recorded ON THIS THREAD. Thread-local because the hook runs on the
    /// panicking thread and [`run_guarded`] catches on that same thread, so two workers (or two
    /// tests) can never read each other's record.
    static LAST_PANIC: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
}

/// C-3: install (once per process) a panic hook that records the message and location for the
/// window to show, then runs the previous hook (stderr, when there is one).
pub fn install_panic_hook() {
    HOOK.call_once(|| {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let location = info
                .location()
                .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
                .unwrap_or_else(|| "unknown location".to_string());
            let message = payload_text(info.payload());
            let record = format!("{message} (at {location})");
            let _ = LAST_PANIC.try_with(|slot| {
                if let Ok(mut slot) = slot.try_borrow_mut() {
                    *slot = Some(record);
                }
            });
            previous(info);
        }));
    });
}

fn payload_text(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "panic with a non-text payload".to_string()
    }
}

/// The last recorded panic (message and location), taken.
fn take_last_panic() -> Option<String> {
    LAST_PANIC
        .try_with(|slot| slot.try_borrow_mut().ok().and_then(|mut s| s.take()))
        .ok()
        .flatten()
}

/// Why a guarded job failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobError {
    /// An ordinary error, already worded for the user.
    Failed(UserError),
    /// The job panicked and the panic was caught (C-1). Text: message and location.
    Panicked(String),
}

impl JobError {
    /// As a user error (a caught panic becomes the "Crashed" row of UI § 7).
    pub fn into_user_error(self) -> UserError {
        match self {
            JobError::Failed(u) => u,
            JobError::Panicked(details) => UserError::new(ErrorKind::Crashed, details),
        }
    }
}

/// C-1: run `f` under ONE `catch_unwind`. See the module doc for why `AssertUnwindSafe` holds.
pub fn run_guarded<T>(f: impl FnOnce() -> Result<T, UserError>) -> Result<T, JobError> {
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(Ok(v)) => Ok(v),
        Ok(Err(e)) => Err(JobError::Failed(e)),
        Err(payload) => {
            let recorded = take_last_panic();
            Err(JobError::Panicked(
                recorded.unwrap_or_else(|| payload_text(payload.as_ref())),
            ))
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Jobs
// ─────────────────────────────────────────────────────────────────────────────

/// Picture bytes that passed [`probe`]: the ONLY form in which the window hands a picture to the
/// library (B-1/B-2: the probe and the decode see the same bytes).
#[derive(Debug, Clone)]
pub struct GuardedImage {
    bytes: Arc<[u8]>,
    probe: Probe,
}

impl GuardedImage {
    /// Probe `bytes` and admit them, or refuse.
    pub fn admit(bytes: Vec<u8>) -> Result<Self, Refusal> {
        let probe = probe(&bytes)?;
        Ok(GuardedImage {
            bytes: bytes.into(),
            probe,
        })
    }

    /// TEST SEAM ONLY: wrap bytes WITHOUT the probe, to prove C-1 catches today's analyzer
    /// panics (SH T1/T2). Never reachable from the window.
    #[cfg(test)]
    pub(crate) fn unchecked(bytes: Vec<u8>, width: u32, height: u32) -> Self {
        GuardedImage {
            bytes: bytes.into(),
            probe: Probe {
                width,
                height,
                format: image::ImageFormat::Png,
            },
        }
    }

    /// The header facts.
    pub fn probe(&self) -> Probe {
        self.probe
    }

    /// The bytes.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// Where a picture comes from.
pub enum LoadSource {
    /// A file the user chose or dropped.
    Path(PathBuf),
    /// Bytes already in memory (the compiled-in sample, or a drop that carried bytes).
    Bytes {
        /// The encoded picture.
        bytes: Vec<u8>,
        /// What to call it.
        name: String,
    },
}

/// A picture that is ready: admitted bytes for the engine, a display copy for the screen.
#[derive(Debug, Clone)]
pub struct LoadedImage {
    /// The admitted bytes (what the engine gets).
    pub image: GuardedImage,
    /// The on-screen copy (≤ 2048 px long side).
    pub display: DisplayImage,
    /// File name (raw; sanitise before display).
    pub name: String,
    /// The path, when it came from a file (for V-4 and the suggested WAV name).
    pub path: Option<PathBuf>,
}

/// Where Play sends sound.
pub enum OutputChoice {
    /// The in-process synth, bundled SoundFont.
    Speakers,
    /// An external MIDI port.
    Midi(MidiTarget),
}

/// One unit of work for the worker.
pub enum JobKind {
    /// Read, check and decode a picture.
    Load(LoadSource),
    /// `render()`, then optionally save the result.
    Render {
        /// The request (built by `Controls::request`).
        request: RenderRequest,
        /// The picture.
        image: GuardedImage,
        /// Save here afterwards (Save WAV with no current render).
        save_to: Option<PathBuf>,
    },
    /// `prepare_play()` then `play()`.
    Play {
        /// The request.
        request: RenderRequest,
        /// The picture.
        image: GuardedImage,
        /// The sink.
        output: OutputChoice,
        /// Stop.
        cancel: CancelToken,
    },
    /// Write an already-rendered piece.
    Save {
        /// The rendered piece.
        audio: Arc<RenderOutput>,
        /// The target.
        path: PathBuf,
    },
}

/// What a job reports back.
#[derive(Debug)]
pub enum Event {
    /// A picture loaded.
    Loaded(Box<LoadedImage>),
    /// `render()` finished.
    Rendered {
        /// The piece, kept as data (the Architect's S123 minimum).
        audio: Arc<RenderOutput>,
        /// Where it was also saved, if asked.
        saved_to: Option<PathBuf>,
    },
    /// `prepare_play()` finished; streaming starts.
    PlayReady {
        /// Composed steps (what `Step` counts).
        total_steps: usize,
        /// Scan positions.
        source_step_count: usize,
    },
    /// The playhead entered this composed step.
    Step(usize),
    /// A sink error during streaming (reported once per play).
    SinkTrouble(UserError),
    /// Streaming ended and the sink was dropped (all sound off).
    PlayEnded {
        /// Ended by Stop / abandon rather than by finishing.
        cancelled: bool,
    },
    /// A WAV was written.
    Saved(PathBuf),
    /// The job failed (a caught panic included).
    Failed(UserError),
}

/// A message from the worker.
#[derive(Debug)]
pub enum WorkerMsg {
    /// An event from job `gen`.
    Event {
        /// Job generation.
        gen: u64,
        /// The event.
        event: Event,
    },
    /// Job `gen` is finished and the worker is free (or gone, if `panicked`).
    Done {
        /// Job generation.
        gen: u64,
        /// The job panicked; the worker thread has exited (W-2).
        panicked: bool,
    },
}

/// A job with its generation.
struct Job {
    gen: u64,
    kind: JobKind,
}

type Notify = Arc<dyn Fn() + Send + Sync>;

/// Read, check and decode a picture (B-1, B-2, B-3, D-1), on the worker.
pub fn run_load(source: LoadSource) -> Result<LoadedImage, UserError> {
    let (bytes, name, path) = match source {
        LoadSource::Path(p) => {
            let bytes = read_bounded(&p).map_err(|r| from_refusal(&r))?;
            let name = p
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| p.display().to_string());
            (bytes, name, Some(p))
        }
        LoadSource::Bytes { bytes, name } => {
            if bytes.len() as u64 > super::ingest::MAX_IMAGE_BYTES {
                return Err(from_refusal(&Refusal::FileTooBig {
                    cap: super::ingest::MAX_IMAGE_BYTES,
                }));
            }
            (bytes, name, None)
        }
    };
    let image = GuardedImage::admit(bytes).map_err(|r| from_refusal(&r))?;
    let display =
        decode_for_display(image.bytes(), &image.probe()).map_err(|r| from_refusal(&r))?;
    Ok(LoadedImage {
        image,
        display,
        name,
        path,
    })
}

/// `render()` over admitted bytes with the GUI's font (the bundled one).
pub fn run_render(
    request: &RenderRequest,
    image: &GuardedImage,
    mappings: &MappingTable,
) -> Result<RenderOutput, UserError> {
    // C-2 (b): the note callback does nothing at all.
    render(
        request,
        ImageInput::Bytes(image.bytes()),
        mappings,
        super::gui_font_source(),
        &mut |_| Ok(()),
    )
    .map_err(|e| from_render_error(&e))
}

/// Build the live sink for `output` (on the worker; the sink never leaves it).
fn open_sink(
    output: OutputChoice,
    request: &RenderRequest,
) -> Result<Box<dyn AudioSink>, UserError> {
    match output {
        OutputChoice::Speakers => {
            use crate::synth_sink::{SynthConfig, SynthSink};
            let config = SynthConfig {
                enable_reverb_and_chorus: request.reverb.is_on(),
                gain: request.gain,
            };
            let synth = SynthSink::new_with_config(super::gui_font_source(), config)
                .map_err(|e| UserError::new(ErrorKind::NoSpeakers, e.to_string()))?;
            Ok(Box::new(SilencingSink::new(synth)))
        }
        OutputChoice::Midi(target) => {
            let midi =
                GuiMidiSink::connect(target).map_err(|e| UserError::new(ErrorKind::MidiGone, e))?;
            Ok(Box::new(SilencingSink::new(midi)))
        }
    }
}

/// Run one job, reporting through `emit`. Every error becomes an `Event::Failed`.
fn run_job(
    kind: JobKind,
    mappings: &MappingTable,
    emit: &dyn Fn(Event),
    sink_live: &AtomicBool,
) -> Result<(), UserError> {
    match kind {
        JobKind::Load(source) => {
            let loaded = run_load(source)?;
            emit(Event::Loaded(Box::new(loaded)));
        }
        JobKind::Render {
            request,
            image,
            save_to,
        } => {
            let audio = Arc::new(run_render(&request, &image, mappings)?);
            let saved_to = match save_to {
                Some(path) => {
                    super::save::write_atomic(&path, audio.sample_rate, &audio.interleaved)
                        .map_err(|e| UserError::new(ErrorKind::SaveFailed, e.to_string()))?;
                    Some(path)
                }
                None => None,
            };
            emit(Event::Rendered { audio, saved_to });
        }
        JobKind::Save { audio, path } => {
            super::save::write_atomic(&path, audio.sample_rate, &audio.interleaved)
                .map_err(|e| UserError::new(ErrorKind::SaveFailed, e.to_string()))?;
            emit(Event::Saved(path));
        }
        JobKind::Play {
            request,
            image,
            output,
            cancel,
        } => {
            let plan = prepare_play(
                &request,
                ImageInput::Bytes(image.bytes()),
                mappings,
                &mut |_| Ok(()),
            )
            .map_err(|e| from_render_error(&e))?;
            emit(Event::PlayReady {
                total_steps: plan.total_steps,
                source_step_count: plan.source_step_count,
            });
            // The CLI's "No steps to play." early return, before any sink exists.
            if plan.source_step_count == 0 || cancel.is_cancelled() {
                emit(Event::PlayEnded {
                    cancelled: cancel.is_cancelled(),
                });
                return Ok(());
            }
            let mut sink = open_sink(output, &request)?;
            sink_live.store(true, Ordering::SeqCst);
            let mut reported = false;
            let result = play(&plan, sink.as_mut(), &cancel, &mut |ev| match ev {
                PlayEvent::Step(s) => emit(Event::Step(s)),
                PlayEvent::SinkError(e) if !reported => {
                    reported = true;
                    emit(Event::SinkTrouble(UserError::new(
                        ErrorKind::MidiGone,
                        e.to_string(),
                    )));
                }
                _ => {}
            });
            // All sound off: SilencingSink's note-offs, then (MIDI) the CC panic, then the
            // synth's stream teardown. Before the UI hears "ended".
            drop(sink);
            sink_live.store(false, Ordering::SeqCst);
            result.map_err(|e| from_render_error(&e))?;
            emit(Event::PlayEnded {
                cancelled: cancel.is_cancelled(),
            });
        }
    }
    Ok(())
}

/// The worker thread's body: one job at a time, each under one `catch_unwind`; exits after a
/// caught panic (W-2) or when the job channel closes.
fn worker_main(
    jobs: Receiver<Job>,
    out: Sender<WorkerMsg>,
    notify: Notify,
    mappings: Arc<MappingTable>,
    sink_live: Arc<AtomicBool>,
) {
    while let Ok(Job { gen, kind }) = jobs.recv() {
        let emit = |event: Event| {
            let _ = out.send(WorkerMsg::Event { gen, event });
            notify();
        };
        let result = run_guarded(|| run_job(kind, &mappings, &emit, &sink_live));
        let panicked = matches!(result, Err(JobError::Panicked(_)));
        if let Err(e) = result {
            emit(Event::Failed(e.into_user_error()));
        }
        // A panic mid-play unwound through the sink's Drop (all sound off); clear the flag.
        sink_live.store(false, Ordering::SeqCst);
        let _ = out.send(WorkerMsg::Done { gen, panicked });
        notify();
        if panicked {
            return; // W-2: a fresh thread takes the next job.
        }
    }
}

struct Worker {
    jobs: Sender<Job>,
    _thread: std::thread::JoinHandle<()>,
}

/// The UI side of the worker: submit, abandon, stop, poll. Owned by the app.
pub struct Controller {
    worker: Option<Worker>,
    events_tx: Sender<WorkerMsg>,
    events: Receiver<WorkerMsg>,
    notify: Notify,
    mappings: Arc<MappingTable>,
    sink_live: Arc<AtomicBool>,
    next_gen: u64,
    /// The generation whose results the UI wants (`None` ⇒ nothing; everything is stale).
    wanted: Option<u64>,
    /// The generation running on the worker, if any (abandoned ones included).
    running: Option<u64>,
    /// T-3: at most ONE job waits while another runs.
    pending: Option<Job>,
    play_cancel: Option<CancelToken>,
    started: Option<Instant>,
}

impl Controller {
    /// A controller over `mappings`; `notify` wakes the UI (e.g. `ctx.request_repaint()`).
    pub fn new(mappings: MappingTable, notify: impl Fn() + Send + Sync + 'static) -> Self {
        let (events_tx, events) = channel();
        Controller {
            worker: None,
            events_tx,
            events,
            notify: Arc::new(notify),
            mappings: Arc::new(mappings),
            sink_live: Arc::new(AtomicBool::new(false)),
            next_gen: 1,
            wanted: None,
            running: None,
            pending: None,
            play_cancel: None,
            started: None,
        }
    }

    fn spawn_worker(&mut self) -> Result<(), String> {
        let (jobs_tx, jobs_rx) = channel();
        let out = self.events_tx.clone();
        let notify = self.notify.clone();
        let mappings = self.mappings.clone();
        let sink_live = self.sink_live.clone();
        let thread = std::thread::Builder::new()
            .name("audiohax-gui-worker".into())
            .spawn(move || worker_main(jobs_rx, out, notify, mappings, sink_live))
            .map_err(|e| e.to_string())?;
        self.worker = Some(Worker {
            jobs: jobs_tx,
            _thread: thread,
        });
        Ok(())
    }

    fn dispatch(&mut self, job: Job) {
        if self.worker.is_none() {
            if let Err(e) = self.spawn_worker() {
                let _ = self.events_tx.send(WorkerMsg::Event {
                    gen: job.gen,
                    event: Event::Failed(UserError::new(ErrorKind::Other, e)),
                });
                return;
            }
        }
        let gen = job.gen;
        let sent = self
            .worker
            .as_ref()
            .map(|w| w.jobs.send(job).is_ok())
            .unwrap_or(false);
        if sent {
            self.running = Some(gen);
            self.started = Some(Instant::now());
        } else {
            // The worker is gone (it can only exit after a panic, which Done reports); retry once
            // on a fresh thread.
            self.worker = None;
            let _ = self.events_tx.send(WorkerMsg::Event {
                gen,
                event: Event::Failed(UserError::new(
                    ErrorKind::Other,
                    "the background worker stopped; try again",
                )),
            });
        }
    }

    /// Submit a job. It runs now if the worker is free, else waits in the single pending slot
    /// (replacing any older pending job). Returns its generation, which becomes the wanted one.
    pub fn submit(&mut self, kind: JobKind) -> u64 {
        let gen = self.next_gen;
        self.next_gen += 1;
        if let JobKind::Play { cancel, .. } = &kind {
            self.play_cancel = Some(cancel.clone());
        }
        self.wanted = Some(gen);
        let job = Job { gen, kind };
        if self.running.is_some() {
            self.pending = Some(job);
        } else {
            self.dispatch(job);
        }
        gen
    }

    /// T-2: abandon whatever is wanted. Its results will be dropped; a playing job is told to
    /// stop; a pending job is discarded. The worker itself finishes what it is doing.
    pub fn abandon(&mut self) {
        self.wanted = None;
        self.pending = None;
        if let Some(c) = self.play_cancel.take() {
            c.cancel();
        }
    }

    /// Stop playback (keeps the job wanted, so its `PlayEnded` arrives).
    pub fn stop_play(&mut self) {
        if let Some(c) = &self.play_cancel {
            c.cancel();
        }
    }

    /// Is a job running on the worker (wanted or abandoned)?
    pub fn is_busy(&self) -> bool {
        self.running.is_some()
    }

    /// Is a submitted job waiting for an ABANDONED one to finish ("Finishing the previous
    /// picture…")?
    pub fn waiting_on_abandoned(&self) -> bool {
        self.pending.is_some() && self.running.is_some() && self.running != self.wanted
    }

    /// How long the current job has been running.
    pub fn running_for(&self) -> Option<Duration> {
        self.running.and(self.started).map(|t| t.elapsed())
    }

    /// Is a live sink open right now?
    pub fn sink_live(&self) -> bool {
        self.sink_live.load(Ordering::SeqCst)
    }

    /// Drain the worker's messages. Returns the events of the WANTED job only; stale ones are
    /// dropped (T-2). Handles `Done`: frees the slot, replaces a worker that exited after a panic
    /// (W-2), and dispatches the pending job.
    pub fn poll(&mut self) -> Vec<Event> {
        let mut out = Vec::new();
        while let Ok(msg) = self.events.try_recv() {
            match msg {
                WorkerMsg::Event { gen, event } => {
                    if Some(gen) == self.wanted {
                        out.push(event);
                    }
                }
                WorkerMsg::Done { gen, panicked } => {
                    if self.running == Some(gen) {
                        self.running = None;
                        self.started = None;
                    }
                    if panicked {
                        self.worker = None;
                    }
                    if Some(gen) == self.wanted {
                        self.play_cancel = None;
                    }
                    if self.running.is_none() {
                        if let Some(job) = self.pending.take() {
                            self.dispatch(job);
                        }
                    }
                }
            }
        }
        out
    }

    /// Window closing: stop any playback and wait (up to `wait`) for the live sink to be dropped,
    /// so its all-sound-off runs before the process exits. Does not wait for an analysis.
    pub fn shutdown(&mut self, wait: Duration) {
        self.pending = None;
        if let Some(c) = self.play_cancel.take() {
            c.cancel();
        }
        self.worker = None; // closes the job channel; the worker exits after its current job
        let t0 = Instant::now();
        while self.sink_live.load(Ordering::SeqCst) && t0.elapsed() < wait {
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}

impl Drop for Controller {
    fn drop(&mut self) {
        self.shutdown(Duration::from_millis(1500));
    }
}

#[cfg(test)]
mod tests {
    use super::super::ingest::fixtures::{solid, structured};
    use super::*;
    use crate::gui::state::Controls;
    use image::ImageFormat;

    fn mappings() -> MappingTable {
        crate::gui::embedded_mappings().expect("embedded mappings parse")
    }

    #[test]
    fn t1_t2_today_s_analyzer_panics_are_caught() {
        install_panic_hook();
        let req = Controls::default().request();
        let m = mappings();
        for (w, h, rgb) in [
            (1, 1, [255, 255, 255]),
            (1, 50, [200, 30, 30]),
            (2, 100, [20, 200, 20]),
        ] {
            let img = GuardedImage::unchecked(solid(w, h, rgb, ImageFormat::Png), w, h);
            let r = run_guarded(|| run_render(&req, &img, &m));
            assert!(
                matches!(r, Err(JobError::Panicked(_))),
                "{w}x{h}: expected a caught panic, got {r:?}"
            );
        }
        // …and the process is alive, and a normal render still works on this thread.
        let ok = GuardedImage::admit(structured(64, 48, ImageFormat::Png)).expect("admit");
        assert!(run_guarded(|| run_render(&req, &ok, &m)).is_ok());
    }

    #[test]
    fn caught_panic_details_carry_the_location() {
        install_panic_hook();
        let r: Result<(), JobError> = run_guarded(|| panic!("boom from a test"));
        match r {
            Err(JobError::Panicked(d)) => {
                assert!(d.contains("boom from a test"), "{d}");
                assert!(d.contains("job.rs"), "{d}");
            }
            other => panic!("expected Panicked, got {other:?}"),
        }
    }

    #[test]
    fn t3_sixteen_square_at_sixteen_voices_renders() {
        let img = GuardedImage::admit(structured(16, 16, ImageFormat::Png)).expect("admit");
        let req = Controls {
            instruments: 16,
            ..Controls::default()
        }
        .request();
        let out = run_render(&req, &img, &mappings()).expect("renders");
        assert!(!out.interleaved.is_empty());
    }

    #[test]
    fn t6_truncated_picture_is_a_plain_error_not_a_panic() {
        let full = structured(64, 64, ImageFormat::Png);
        let half = full[..full.len() / 2].to_vec();
        let img = GuardedImage::admit(half).expect("header admitted");
        let r = run_guarded(|| run_render(&Controls::default().request(), &img, &mappings()));
        match r {
            Err(JobError::Failed(u)) => assert_eq!(u.kind, ErrorKind::NotAPicture),
            other => panic!("expected a plain decode error, got {other:?}"),
        }
    }

    #[test]
    fn t10_jpeg_named_png_renders_through_bytes() {
        let dir = crate::gui::test_scratch_dir("job-t10");
        let p = dir.join("x.png");
        std::fs::write(&p, structured(80, 60, ImageFormat::Jpeg)).expect("write");
        let loaded = run_load(LoadSource::Path(p)).expect("loads");
        assert_eq!(loaded.image.probe().format, ImageFormat::Jpeg);
        assert!(run_render(&Controls::default().request(), &loaded.image, &mappings()).is_ok());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn gui_render_equals_the_library_render_with_the_bundled_font() {
        // The GUI's render path is `render()` with the bundled font and the embedded mappings:
        // byte-identical samples to calling the library directly the way the CLI does.
        let bytes = structured(96, 64, ImageFormat::Png);
        let img = GuardedImage::admit(bytes.clone()).expect("admit");
        let req = Controls {
            seed: Some(42),
            ..Controls::default()
        }
        .request();
        let m = mappings();
        let gui = run_render(&req, &img, &m).expect("gui render");
        let lib = render(
            &req,
            ImageInput::Bytes(&bytes),
            &m,
            crate::synth_sink::SoundFontSource::Bundled,
            &mut |_| Ok(()),
        )
        .expect("lib render");
        assert_eq!(gui.sample_rate, lib.sample_rate);
        assert_eq!(gui.interleaved, lib.interleaved);
        assert_eq!(gui.events, lib.events);
    }

    #[test]
    fn sample_picture_loads_and_its_display_copy_is_bounded() {
        let l = run_load(LoadSource::Bytes {
            bytes: crate::gui::SAMPLE_IMAGE.to_vec(),
            name: crate::gui::SAMPLE_NAME.into(),
        })
        .expect("sample loads");
        assert!(l.display.width.max(l.display.height) <= super::super::ingest::DISPLAY_MAX_SIDE);
        assert!(l.path.is_none());
    }

    fn drain_until_done(c: &mut Controller, timeout: Duration) -> Vec<Event> {
        let t0 = Instant::now();
        let mut all = Vec::new();
        loop {
            all.extend(c.poll());
            if !c.is_busy() && c.pending.is_none() {
                return all;
            }
            assert!(
                t0.elapsed() < timeout,
                "worker did not finish in {timeout:?}"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    #[test]
    fn controller_runs_one_job_at_a_time_and_drops_stale_results() {
        let mut c = Controller::new(mappings(), || {});
        let img = GuardedImage::admit(structured(64, 48, ImageFormat::Png)).expect("admit");
        let req = Controls::default().request();
        let g1 = c.submit(JobKind::Render {
            request: req.clone(),
            image: img.clone(),
            save_to: None,
        });
        assert!(c.is_busy());
        // Abandon, then submit while the abandoned job may still run: it waits (T-3).
        c.abandon();
        let g2 = c.submit(JobKind::Render {
            request: req,
            image: img,
            save_to: None,
        });
        assert!(g2 > g1);
        let events = drain_until_done(&mut c, Duration::from_secs(60));
        let rendered = events
            .iter()
            .filter(|e| matches!(e, Event::Rendered { .. }))
            .count();
        assert_eq!(rendered, 1, "only the wanted job's result arrives");
    }

    #[test]
    fn w2_a_panicking_job_is_reported_and_the_next_job_gets_a_fresh_worker() {
        install_panic_hook();
        let mut c = Controller::new(mappings(), || {});
        let bad = GuardedImage::unchecked(solid(1, 1, [255, 255, 255], ImageFormat::Png), 1, 1);
        c.submit(JobKind::Render {
            request: Controls::default().request(),
            image: bad,
            save_to: None,
        });
        let ev = drain_until_done(&mut c, Duration::from_secs(30));
        assert!(ev.iter().any(|e| matches!(
            e,
            Event::Failed(UserError {
                kind: ErrorKind::Crashed,
                ..
            })
        )));
        assert!(c.worker.is_none(), "the panicked worker was retired");
        let ok = GuardedImage::admit(structured(64, 48, ImageFormat::Png)).expect("admit");
        c.submit(JobKind::Render {
            request: Controls::default().request(),
            image: ok,
            save_to: None,
        });
        let ev = drain_until_done(&mut c, Duration::from_secs(60));
        assert!(ev.iter().any(|e| matches!(e, Event::Rendered { .. })));
    }

    #[test]
    fn play_prepares_on_the_worker_and_a_stop_before_streaming_opens_no_sink() {
        // Exercises prepare_play() on the worker and the Stop path WITHOUT opening an audio
        // device: the token is cancelled before the sink would be built.
        let mut c = Controller::new(mappings(), || {});
        let img = GuardedImage::admit(structured(64, 48, ImageFormat::Png)).expect("admit");
        let cancel = CancelToken::default();
        cancel.cancel();
        c.submit(JobKind::Play {
            request: Controls::default().request(),
            image: img,
            output: OutputChoice::Speakers,
            cancel,
        });
        let ev = drain_until_done(&mut c, Duration::from_secs(60));
        let ready = ev.iter().find_map(|e| match e {
            Event::PlayReady {
                total_steps,
                source_step_count,
            } => Some((*total_steps, *source_step_count)),
            _ => None,
        });
        let (total, scan) = ready.expect("PlayReady");
        assert!(total > 0);
        assert_eq!(scan, 40, "default --steps 40 => 40 scan positions");
        assert!(ev
            .iter()
            .any(|e| matches!(e, Event::PlayEnded { cancelled: true })));
        assert!(!c.sink_live());
    }

    #[test]
    fn render_then_save_writes_the_same_samples_render_returned() {
        let dir = crate::gui::test_scratch_dir("job-save");
        let target = dir.join("piece.wav");
        let mut c = Controller::new(mappings(), || {});
        let img = GuardedImage::admit(structured(64, 48, ImageFormat::Png)).expect("admit");
        c.submit(JobKind::Render {
            request: Controls::default().request(),
            image: img,
            save_to: Some(target.clone()),
        });
        let ev = drain_until_done(&mut c, Duration::from_secs(60));
        let audio = ev
            .iter()
            .find_map(|e| match e {
                Event::Rendered { audio, saved_to } => {
                    assert_eq!(saved_to.as_deref(), Some(target.as_path()));
                    Some(audio.clone())
                }
                _ => None,
            })
            .expect("rendered");
        let mut r = hound::WavReader::open(&target).expect("wav");
        assert_eq!(r.spec().sample_rate, audio.sample_rate);
        let written: Vec<i16> = r.samples::<i16>().map(|s| s.expect("sample")).collect();
        let expected: Vec<i16> = audio
            .interleaved
            .iter()
            .map(|s| (s.clamp(-1.0, 1.0) * i16::MAX as f32).round() as i16)
            .collect();
        assert_eq!(written, expected);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
