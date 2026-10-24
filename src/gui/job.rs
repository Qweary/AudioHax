//! The one worker thread (security review `s123-SH` § 4.2–§ 4.4; lead ruling L-3 carried).
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
//! * **L-21.** The worker keeps the LAST picture analysis ([`AnalysisCache`]), so Render, Play and
//!   Save WAV on the same picture and the same Voices analyse it once instead of once per press
//!   (about 38 s each on a 24 MP photo, `s123-SH2` N-6). The cache lives on the worker thread
//!   only; it adds no job and no thread. A caught panic ends the thread and so drops the cache
//!   with it, which is why borrowing it mutably inside the `AssertUnwindSafe` closure is sound.

use super::ingest::{decode_for_display, probe, read_bounded, DisplayImage, Probe, Refusal};
use super::sinks::{GuiMidiSink, MidiTarget, SilencingSink};
use super::text::{from_refusal, from_render_error, ErrorKind, UserError};
use crate::cli::PipelineArgs;
use crate::composition::ImageUnderstanding;
use crate::engine::AudioSink;
use crate::mapping_loader::MappingTable;
use crate::pure_analysis::PureAnalysisSource;
use crate::render::{
    acquire_shrunk, play, prepare_play_with_source, render_shrunk, render_with_source, CancelToken,
    ImageInput, PlayEvent, PlayPlan, RenderError, RenderOutput, RenderRequest,
};
use crate::shrink::Shrink;
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
///
/// S125: it also carries the shrink the music is made at ([`Self::with_shrink`]; `Shrink::Full`
/// unless set). The probe, and so every admission cap, is always the ORIGINAL picture's (SH5 C-3):
/// the shrink is applied by the library after the decode, in `render::acquire_shrunk`.
#[derive(Debug, Clone)]
pub struct GuardedImage {
    bytes: Arc<[u8]>,
    probe: Probe,
    shrink: Shrink,
}

impl GuardedImage {
    /// Probe `bytes` and admit them, or refuse.
    pub fn admit(bytes: Vec<u8>) -> Result<Self, Refusal> {
        let probe = probe(&bytes)?;
        Ok(GuardedImage {
            bytes: bytes.into(),
            probe,
            shrink: Shrink::Full,
        })
    }

    /// The same admitted picture, to be made into music at `shrink` (S125). The bytes are shared,
    /// not copied, so the analysis cache still recognises the picture.
    pub fn with_shrink(mut self, shrink: Shrink) -> Self {
        self.shrink = shrink;
        self
    }

    /// The shrink the music is made at.
    pub fn shrink(&self) -> Shrink {
        self.shrink
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
            shrink: Shrink::Full,
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
    render_shrunk(
        request,
        image.shrink(),
        ImageInput::Bytes(image.bytes()),
        mappings,
        super::gui_font_source(),
        &mut |_| Ok(()),
    )
    .map_err(|e| from_render_error(&e))
}

// ─────────────────────────────────────────────────────────────────────────────
// L-21: one cached analysis per picture
// ─────────────────────────────────────────────────────────────────────────────

/// One analysed picture: what the library's `render::acquire` produces.
struct Analysed {
    /// The picture it was made from (the same `Arc`, so identity is a pointer comparison and the
    /// bytes cannot be freed and reused while the entry exists).
    image: Arc<[u8]>,
    /// Everything `acquire` reads from the request: the instrument count, bar thickness and step
    /// count all change the extracted features. Keyed on the whole `PipelineArgs` so that any
    /// later field `acquire` starts to read also invalidates the entry.
    pipeline: PipelineArgs,
    /// S125 (SH5 C-7): the shrink it was analysed at. A different level is a different analysis.
    shrink: Shrink,
    source: PureAnalysisSource,
    understanding: ImageUnderstanding,
}

/// L-21: the most recent picture analysis, kept on the worker thread.
///
/// In `src/render.rs`, `render()` is `validate`, then `acquire`, then `render_with_source`; and
/// `prepare_play()` is `acquire`, then `prepare_play_with_source`. [`analyse`] calls the same
/// `render::acquire` (public since S124; S123 had to repeat its three steps here). It sets no
/// thread-local register (the seed, expression and cadence registers are all set inside
/// `compose()`, which both `*_with_source` entries call), so reusing its result changes nothing
/// about the piece.
/// `cached_render_and_play_are_byte_identical_to_the_library` pins that.
#[derive(Default)]
pub struct AnalysisCache {
    entry: Option<Analysed>,
    analyses: usize,
}

/// `render::acquire_shrunk` over the admitted bytes (decode, shrink, understand, extract). Its
/// notes are dropped, as every GUI job drops them (C-2 (b)).
fn analyse(
    pipeline: &PipelineArgs,
    shrink: Shrink,
    bytes: &[u8],
) -> Result<(PureAnalysisSource, ImageUnderstanding), RenderError> {
    acquire_shrunk(pipeline, shrink, ImageInput::Bytes(bytes), &mut |_| Ok(()))
}

impl AnalysisCache {
    /// Forget the cached analysis (a new picture was loaded).
    pub fn clear(&mut self) {
        self.entry = None;
    }

    /// How many times this cache has run the analysis (misses). For tests.
    pub fn analyses(&self) -> usize {
        self.analyses
    }

    /// The analysis of `image` under `pipeline`, from the cache or freshly made.
    fn get(
        &mut self,
        pipeline: &PipelineArgs,
        image: &GuardedImage,
    ) -> Result<(&PureAnalysisSource, &ImageUnderstanding), RenderError> {
        let hit = self.entry.as_ref().is_some_and(|e| {
            Arc::ptr_eq(&e.image, &image.bytes)
                && e.pipeline == *pipeline
                && e.shrink == image.shrink
        });
        if !hit {
            // Drop the old entry BEFORE analysing, so two analyses never sit in memory at once.
            self.entry = None;
            let (source, understanding) = analyse(pipeline, image.shrink, image.bytes())?;
            self.analyses += 1;
            self.entry = Some(Analysed {
                image: image.bytes.clone(),
                pipeline: pipeline.clone(),
                shrink: image.shrink,
                source,
                understanding,
            });
        }
        match self.entry.as_ref() {
            Some(e) => Ok((&e.source, &e.understanding)),
            None => Err(RenderError::Refused(
                "internal: the picture analysis was not kept".into(),
            )),
        }
    }

    /// `render()`, reusing the cached analysis. Byte-identical to [`run_render`].
    pub fn render(
        &mut self,
        request: &RenderRequest,
        image: &GuardedImage,
        mappings: &MappingTable,
    ) -> Result<RenderOutput, UserError> {
        let go = |cache: &mut Self| -> Result<RenderOutput, RenderError> {
            // `render()` validates BEFORE it analyses; keep that order for the same errors.
            request.validate()?;
            let (source, understanding) = cache.get(&request.pipeline, image)?;
            render_with_source(
                request,
                source,
                Some(understanding),
                mappings,
                super::gui_font_source(),
                &mut |_| Ok(()),
            )
        };
        go(self).map_err(|e| from_render_error(&e))
    }

    /// `prepare_play()`, reusing the cached analysis. Byte-identical to the library's.
    pub fn prepare_play(
        &mut self,
        request: &RenderRequest,
        image: &GuardedImage,
        mappings: &MappingTable,
    ) -> Result<PlayPlan, UserError> {
        let go = |cache: &mut Self| -> Result<PlayPlan, RenderError> {
            let (source, understanding) = cache.get(&request.pipeline, image)?;
            prepare_play_with_source(request, source, Some(understanding), mappings, &mut |_| {
                Ok(())
            })
        };
        go(self).map_err(|e| from_render_error(&e))
    }
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
    cache: &mut AnalysisCache,
) -> Result<(), UserError> {
    match kind {
        JobKind::Load(source) => {
            let loaded = run_load(source)?;
            // A new picture is showing: the old analysis can never be asked for again.
            cache.clear();
            emit(Event::Loaded(Box::new(loaded)));
        }
        JobKind::Render {
            request,
            image,
            save_to,
        } => {
            let audio = Arc::new(cache.render(&request, &image, mappings)?);
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
            let plan = cache.prepare_play(&request, &image, mappings)?;
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
    // L-21: lives as long as this thread; a caught panic ends the thread and drops it.
    let mut cache = AnalysisCache::default();
    while let Ok(Job { gen, kind }) = jobs.recv() {
        let emit = |event: Event| {
            let _ = out.send(WorkerMsg::Event { gen, event });
            notify();
        };
        let result = run_guarded(|| run_job(kind, &mappings, &emit, &sink_live, &mut cache));
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
    use crate::render::{prepare_play, render};
    use image::ImageFormat;

    /// The library's own `render()` and `prepare_play()`, uncached: the reference.
    fn library(req: &RenderRequest, bytes: &[u8], m: &MappingTable) -> (RenderOutput, PlayPlan) {
        let out = render(
            req,
            ImageInput::Bytes(bytes),
            m,
            crate::synth_sink::SoundFontSource::Bundled,
            &mut |_| Ok(()),
        )
        .expect("library render");
        let plan = prepare_play(req, ImageInput::Bytes(bytes), m, &mut |_| Ok(())).expect("prep");
        (out, plan)
    }

    fn assert_same_render(a: &RenderOutput, b: &RenderOutput, what: &str) {
        assert_eq!(a.sample_rate, b.sample_rate, "{what}: rate");
        assert_eq!(a.interleaved, b.interleaved, "{what}: samples");
        assert_eq!(a.events, b.events, "{what}: events");
        assert_eq!(a.summary, b.summary, "{what}: summary");
    }

    #[test]
    fn cached_render_and_play_are_byte_identical_to_the_library() {
        // L-21's condition: the cached path must give exactly what the uncached library gives,
        // on a miss, on a hit, after Voices changes (a new analysis), and back again.
        let bytes = structured(96, 64, ImageFormat::Png);
        let img = GuardedImage::admit(bytes.clone()).expect("admit");
        let m = mappings();
        let mut cache = AnalysisCache::default();
        let four = Controls {
            seed: Some(42),
            ..Controls::default()
        };
        let three = Controls {
            instruments: 3,
            reverb: false,
            gain: 0.8,
            ..four
        };
        let (lib4, plan4) = library(&four.request(), &bytes, &m);
        let (lib3, plan3) = library(&three.request(), &bytes, &m);

        let miss = cache.render(&four.request(), &img, &m).expect("miss");
        assert_same_render(&miss, &lib4, "4 voices, miss");
        assert_eq!(cache.analyses(), 1);
        let hit = cache.render(&four.request(), &img, &m).expect("hit");
        assert_same_render(&hit, &lib4, "4 voices, hit");
        assert_eq!(
            cache.prepare_play(&four.request(), &img, &m).expect("play"),
            plan4
        );
        assert_eq!(cache.analyses(), 1, "Render, Render, Play: one analysis");

        let other = cache.render(&three.request(), &img, &m).expect("3 voices");
        assert_same_render(&other, &lib3, "3 voices");
        assert_eq!(
            cache
                .prepare_play(&three.request(), &img, &m)
                .expect("play 3"),
            plan3
        );
        assert_eq!(cache.analyses(), 2, "Voices changed: analysed again, once");
        let back = cache.render(&four.request(), &img, &m).expect("back to 4");
        assert_same_render(&back, &lib4, "back to 4 voices");
        assert_eq!(cache.analyses(), 3);

        // Room and Level and the Variation are NOT analysis inputs: no new analysis.
        let louder = Controls {
            gain: 1.5,
            seed: Some(7),
            ..four
        };
        let (lib_louder, _) = library(&louder.request(), &bytes, &m);
        let r = cache.render(&louder.request(), &img, &m).expect("louder");
        assert_same_render(&r, &lib_louder, "gain/seed changed");
        assert_eq!(cache.analyses(), 3);

        // Potency control: the key matters. A cache that served the 4-voice analysis to a 3-voice
        // request would be wrong, because the two pieces differ.
        assert_ne!(
            lib4.events, lib3.events,
            "control: Voices changes the piece"
        );
        assert_ne!(plan4, plan3);
    }

    /// S125 (SH5 C-7 / forcing row S7; UI4 § 5.3): the cache keys on the shrink. Full, then 50 %,
    /// then Full again on the same picture are three analyses, each byte-identical to the library's
    /// own `render_shrunk`; a cache that served the full-size analysis to the 50 % request would
    /// play the wrong piece with no error, and the potency check below shows the two differ.
    #[test]
    fn s125_the_cache_keys_on_the_shrink_and_matches_the_library() {
        let bytes = structured(96, 64, ImageFormat::Png);
        let full = GuardedImage::admit(bytes.clone()).expect("admit");
        assert_eq!(
            full.shrink(),
            Shrink::Full,
            "admitted pictures start at Full size"
        );
        let half = full.clone().with_shrink(Shrink::Percent50);
        assert_eq!(
            half.probe(),
            full.probe(),
            "the probe stays the ORIGINAL picture's"
        );
        let m = mappings();
        let req = Controls {
            seed: Some(7),
            ..Controls::default()
        }
        .request();
        let lib = |s: Shrink| {
            crate::render::render_shrunk(
                &req,
                s,
                ImageInput::Bytes(&bytes),
                &m,
                crate::synth_sink::SoundFontSource::Bundled,
                &mut |_| Ok(()),
            )
            .expect("library render")
        };
        let lib_full = lib(Shrink::Full);
        let lib_half = lib(Shrink::Percent50);
        let mut cache = AnalysisCache::default();
        assert_same_render(
            &cache.render(&req, &full, &m).expect("full"),
            &lib_full,
            "Full",
        );
        assert_eq!(cache.analyses(), 1);
        assert_same_render(
            &cache.render(&req, &half, &m).expect("50%"),
            &lib_half,
            "50%",
        );
        assert_eq!(cache.analyses(), 2, "a shrink change is a new analysis");
        assert_same_render(
            &cache.render(&req, &half, &m).expect("50% again"),
            &lib_half,
            "50% hit",
        );
        assert_eq!(cache.analyses(), 2, "same shrink: a hit");
        assert_same_render(
            &cache.render(&req, &full, &m).expect("back to Full"),
            &lib_full,
            "back to Full",
        );
        assert_eq!(cache.analyses(), 3);
        // The uncached window path agrees too.
        assert_same_render(
            &run_render(&req, &half, &m).expect("run"),
            &lib_half,
            "run_render",
        );
        // And Full through the shrink entry is the pre-S125 `render`, byte for byte.
        let (plain, _) = library(&req, &bytes, &m);
        assert_same_render(&lib_full, &plain, "render_shrunk(Full) == render");
        // Potency: the 50 % piece is a different piece.
        assert_ne!(lib_full.interleaved, lib_half.interleaved);
    }

    #[test]
    fn a_different_picture_with_the_same_bytes_is_still_a_new_analysis() {
        // Identity is the loaded picture (its `Arc`), not the byte content: a re-load is analysed
        // afresh, which is never wrong, only slower.
        let bytes = structured(64, 48, ImageFormat::Png);
        let a = GuardedImage::admit(bytes.clone()).expect("admit");
        let b = GuardedImage::admit(bytes).expect("admit");
        let m = mappings();
        let mut cache = AnalysisCache::default();
        let req = Controls::default().request();
        let ra = cache.render(&req, &a, &m).expect("a");
        let rb = cache.render(&req, &b, &m).expect("b");
        assert_eq!(cache.analyses(), 2);
        assert_same_render(&ra, &rb, "same bytes");
        cache.clear();
        let _ = cache.render(&req, &b, &m).expect("after clear");
        assert_eq!(cache.analyses(), 3);
    }

    #[test]
    fn a_refused_request_is_refused_before_the_analysis_as_in_render() {
        // `render()` validates first; so does the cached path (same error, no analysis run).
        let img = GuardedImage::admit(structured(64, 48, ImageFormat::Png)).expect("admit");
        let mut req = Controls::default().request();
        req.loop_cue = true;
        req.gain = 2.0;
        let mut cache = AnalysisCache::default();
        let e = cache.render(&req, &img, &mappings()).expect_err("refused");
        assert_eq!(e.kind, ErrorKind::LoopNeedsUnity);
        assert_eq!(cache.analyses(), 0);
    }

    /// `s123-SH` T22 (`s123-05` TODO-7): what a 6000 x 4000 photo costs, and what L-21 saves.
    /// Documents cost; asserts only that each step finishes and the cached second press is
    /// cheaper. Run with `cargo test --features gui --release -- --ignored t22`.
    #[test]
    #[ignore = "cost documentation: about a minute and several hundred MiB on a laptop"]
    fn t22_cost_of_a_24_megapixel_photo() {
        let t = Instant::now();
        let bytes = structured(6000, 4000, ImageFormat::Jpeg);
        eprintln!(
            "T22 fixture: {} bytes JPEG, made in {:?}",
            bytes.len(),
            t.elapsed()
        );
        let t = Instant::now();
        let loaded = run_load(LoadSource::Bytes {
            bytes,
            name: "t22.jpg".into(),
        })
        .expect("a 24 MP photo is admitted");
        eprintln!("T22 load (read, probe, display decode): {:?}", t.elapsed());
        let m = mappings();
        let req = Controls::default().request();
        let mut cache = AnalysisCache::default();
        let t = Instant::now();
        let first = cache.render(&req, &loaded.image, &m).expect("render");
        let miss = t.elapsed();
        eprintln!("T22 first Render (analysis + compose + synth): {miss:?}");
        let t = Instant::now();
        let second = cache.render(&req, &loaded.image, &m).expect("render again");
        let hit = t.elapsed();
        eprintln!("T22 second Render, cached analysis (compose + synth): {hit:?}");
        let t = Instant::now();
        let _plan = cache
            .prepare_play(&req, &loaded.image, &m)
            .expect("prepare");
        eprintln!("T22 Play's prepare, cached analysis: {:?}", t.elapsed());
        assert_eq!(first.interleaved, second.interleaved);
        assert_eq!(cache.analyses(), 1);
        assert!(
            hit < miss,
            "the cached press must be cheaper ({hit:?} vs {miss:?})"
        );
    }

    fn mappings() -> MappingTable {
        crate::gui::embedded_mappings().expect("embedded mappings parse")
    }

    /// SH T1/T2 (C-1). RE-AIMED in S125 (lead ruling L-61, `s125-SH5` B-5): this test used the
    /// analyser's 1×1 / 1×50 / 2×100 panics as its panic source, and D-45 cured them. The panic is
    /// now INJECTED inside the guarded job: once before any work, and once after a real render
    /// has run inside the same job. What it proves is unchanged: a panic inside a job comes back
    /// as `JobError::Panicked` carrying its message, and the same thread then renders normally.
    /// If `run_guarded`'s `catch_unwind` is removed, the panic escapes and this test fails.
    #[test]
    fn t1_t2_today_s_analyzer_panics_are_caught() {
        install_panic_hook();
        let req = Controls::default().request();
        let m = mappings();
        let ok = GuardedImage::admit(structured(64, 48, ImageFormat::Png)).expect("admit");

        let before_work: Result<RenderOutput, JobError> =
            run_guarded(|| panic!("t1 injected panic before any work"));
        match before_work {
            Err(JobError::Panicked(d)) => assert!(d.contains("t1 injected panic"), "{d}"),
            other => panic!("expected a caught panic, got {other:?}"),
        }

        let after_a_render: Result<RenderOutput, JobError> = run_guarded(|| {
            let out = run_render(&req, &ok, &m)?;
            assert!(!out.interleaved.is_empty());
            panic!("t2 injected panic after a real render")
        });
        match after_a_render {
            Err(JobError::Panicked(d)) => assert!(d.contains("t2 injected panic"), "{d}"),
            other => panic!("expected a caught panic, got {other:?}"),
        }

        // …and the process is alive, and a normal render still works on this thread.
        assert!(run_guarded(|| run_render(&req, &ok, &m)).is_ok());
    }

    /// `s125-SH5` B-5 (ii): the three pictures T1/T2 used to panic on (D-45 classes a and b) now
    /// come back from the window's render path as a typed error, never as `Panicked`.
    #[test]
    fn t1_t2_the_pictures_that_used_to_panic_now_return_a_typed_error() {
        install_panic_hook();
        let req = Controls::default().request();
        let m = mappings();
        for (w, h, rgb) in [
            (1, 1, [255, 255, 255]),
            (1, 50, [200, 30, 30]),
            (2, 100, [20, 200, 20]),
        ] {
            let img = GuardedImage::unchecked(solid(w, h, rgb, ImageFormat::Png), w, h);
            match run_guarded(|| run_render(&req, &img, &m)) {
                Err(JobError::Failed(u)) => {
                    assert_eq!(u.kind, ErrorKind::TooManyVoices, "{w}x{h}: {u:?}")
                }
                other => panic!("{w}x{h}: expected a typed error, got {other:?}"),
            }
        }
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

    /// Stale-result dropping: after an abandon, only the wanted (newer) job's result reaches the
    /// window. This counts delivered results only, so it cannot see T-3 (one job alive) broken;
    /// `tests/gui_sealA_s123.rs` `t3_a_job_submitted_while_an_abandoned_one_runs_waits_in_the_one_pending_slot`
    /// pins T-3.
    #[test]
    fn controller_delivers_only_the_wanted_result_after_an_abandon() {
        let mut c = Controller::new(mappings(), || {});
        let img = GuardedImage::admit(structured(64, 48, ImageFormat::Png)).expect("admit");
        let req = Controls::default().request();
        let g1 = c.submit(JobKind::Render {
            request: req.clone(),
            image: img.clone(),
            save_to: None,
        });
        assert!(c.is_busy());
        // Abandon, then submit while the abandoned job may still run; its result must be dropped.
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

    /// W-2. RE-AIMED in S125 (lead ruling L-61, `s125-SH5` B-5): the panic used to come from the
    /// analyser on a 1×1 picture, which D-45 cured. It is now INJECTED through the controller's
    /// wake callback, which the worker calls inside the guarded job when the job emits its first
    /// event. It fires once. The assertions are unchanged, plus one: the reported crash carries
    /// the injected message, so it cannot be some other failure.
    #[test]
    fn w2_a_panicking_job_is_reported_and_the_next_job_gets_a_fresh_worker() {
        install_panic_hook();
        let armed = Arc::new(AtomicBool::new(true));
        let trigger = armed.clone();
        let mut c = Controller::new(mappings(), move || {
            if trigger.swap(false, Ordering::SeqCst) {
                panic!("w2 injected panic inside a worker job");
            }
        });
        let img = GuardedImage::admit(structured(64, 48, ImageFormat::Png)).expect("admit");
        c.submit(JobKind::Render {
            request: Controls::default().request(),
            image: img,
            save_to: None,
        });
        let ev = drain_until_done(&mut c, Duration::from_secs(60));
        assert!(!armed.load(Ordering::SeqCst), "the injected panic fired");
        assert!(ev.iter().any(|e| matches!(
            e,
            Event::Failed(UserError {
                kind: ErrorKind::Crashed,
                details,
            }) if details.contains("w2 injected panic")
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
