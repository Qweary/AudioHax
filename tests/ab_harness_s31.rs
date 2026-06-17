//! tests/ab_harness_s31.rs — integration test for the S31 audio A/B harness.
//!
//! Exercises the END-TO-END library path the `render --wav` binary uses: pure-Rust
//! image analysis → engine decisions → absolutely-timed MIDI events → offline
//! (no-cpal) synth render → WAV bytes. The bin's `run_render_wav` is a thin wrapper
//! over exactly these public library calls, so testing the library path proves the
//! harness without needing an audio device.
//!
//! Requires the default feature set (`synth` + `pure-analysis`); runs headlessly (no
//! cpal device, no OpenCV). Skipped under `--no-default-features`.
#![cfg(all(feature = "synth", feature = "pure-analysis"))]

use std::path::PathBuf;

use audiohax::cli::{pipeline_to_engine_config, PipelineArgs};
use audiohax::engine::{FeatureSource, PipelineEngine};
use audiohax::mapping_loader::load_mappings;
use audiohax::pure_analysis::{load_pure_image, PureAnalysisSource, PureImageSource};
use audiohax::synth_sink::{
    render_events_to_stereo, write_stereo_wav, MidiCmd, SoundFontSource, SynthConfig,
    TimedMidiEvent,
};

/// Build the same deterministic, absolutely-timed event list `run_render_wav` builds
/// for a given image (NO jitter — that is the determinism contract of the offline path).
fn events_for_image(image: &str, pipeline: &PipelineArgs) -> Vec<TimedMidiEvent> {
    let mappings = load_mappings("assets/mappings.json").expect("mappings load");
    let engine_config = pipeline_to_engine_config(pipeline);
    let instrument_count = engine_config.num_instruments;
    let ms_per_step = engine_config.ms_per_step;

    let img =
        load_pure_image(&PureImageSource::UserPath(PathBuf::from(image))).expect("load image");
    let source = PureAnalysisSource::extract(
        &img,
        instrument_count,
        engine_config.bar_thickness_frac,
        pipeline.steps,
        None,
    )
    .expect("extract features");

    let mut engine = PipelineEngine::new(mappings, engine_config);
    engine.set_features_global(&source.global_features());

    let mut events: Vec<TimedMidiEvent> = Vec::new();
    for i in 0..instrument_count {
        let ch = (i % 16) as u8;
        let prog = ((i * 7) % 128) as u8;
        events.push(TimedMidiEvent {
            at_ms: 0,
            cmd: MidiCmd::ProgramChange {
                channel: ch,
                program: prog,
            },
        });
    }
    for step_idx in 0..source.step_count() {
        let step_base_ms = step_idx as u64 * ms_per_step;
        for dec in engine.decide_step(&source, step_idx) {
            for ev in &dec.events {
                let on_ms = step_base_ms + ev.offset_ms;
                events.push(TimedMidiEvent {
                    at_ms: on_ms,
                    cmd: MidiCmd::NoteOn {
                        channel: dec.channel,
                        note: ev.note,
                        velocity: ev.velocity,
                    },
                });
                events.push(TimedMidiEvent {
                    at_ms: on_ms + ev.hold_ms,
                    cmd: MidiCmd::NoteOff {
                        channel: dec.channel,
                        note: ev.note,
                    },
                });
            }
        }
    }
    events
}

/// A short pipeline so the test renders fast (fewer steps) yet still produces real
/// musical events from the real composition path.
fn fast_pipeline() -> PipelineArgs {
    PipelineArgs {
        steps: 6,
        ..PipelineArgs::default()
    }
}

#[test]
fn render_wav_end_to_end_is_nonsilent() {
    let events = events_for_image("assets/images/example.jpg", &fast_pipeline());
    assert!(
        events.len() > 1,
        "the composition must emit real note events (got {})",
        events.len()
    );

    let interleaved = render_events_to_stereo(
        SoundFontSource::Bundled,
        SynthConfig::default(),
        44_100,
        events,
        1_000,
    )
    .expect("offline render");

    let peak = interleaved.iter().fold(0.0f32, |m, &s| m.max(s.abs()));
    assert!(
        peak > 1e-3,
        "the rendered composition must be audible; peak={peak}"
    );

    // And it writes a real, re-readable stereo WAV.
    let mut out = std::env::temp_dir();
    out.push("audiohax_s31_e2e.wav");
    write_stereo_wav(&out, 44_100, &interleaved).expect("write wav");
    let reader = hound::WavReader::open(&out).expect("reopen");
    assert_eq!(reader.spec().channels, 2);
    assert_eq!(reader.spec().sample_rate, 44_100);
    let _ = std::fs::remove_file(&out);
}

#[test]
fn render_wav_same_composition_same_config_is_byte_identical() {
    // The A/B determinism contract, stated precisely: HOLD THE COMPOSITION (event list)
    // CONSTANT and the RENDER is byte-identical across runs — so the only audible
    // variable in an A/B sweep is the synth config (soundfont/reverb/gain), never render
    // jitter. (Note: the upstream engine's plan selection uses thread_rng at
    // set_features_global time, so two *separate* engine runs over the same image may
    // pick different progressions — that is frozen engine behavior, outside this audio
    // change. The A/B harness renders ONE captured composition through each config, which
    // is exactly this test.)
    let events = events_for_image("assets/images/example.jpg", &fast_pipeline());
    let a = render_events_to_stereo(
        SoundFontSource::Bundled,
        SynthConfig::default(),
        44_100,
        events.clone(),
        1_000,
    )
    .expect("render a");
    let b = render_events_to_stereo(
        SoundFontSource::Bundled,
        SynthConfig::default(),
        44_100,
        events,
        1_000,
    )
    .expect("render b");

    assert_eq!(a.len(), b.len());
    assert_eq!(
        a.iter().map(|s| s.to_bits()).collect::<Vec<_>>(),
        b.iter().map(|s| s.to_bits()).collect::<Vec<_>>(),
        "same composition + same config must render byte-identically (A/B determinism)"
    );
}

#[test]
fn render_wav_reverb_off_differs_from_default() {
    // Confirms the `--reverb off` config flows through the harness and changes audio,
    // holding the SAME captured composition constant across both renders (true A/B).
    let events = events_for_image("assets/images/example.jpg", &fast_pipeline());
    let wet = render_events_to_stereo(
        SoundFontSource::Bundled,
        SynthConfig::default(),
        44_100,
        events.clone(),
        1_000,
    )
    .expect("wet");
    let dry = render_events_to_stereo(
        SoundFontSource::Bundled,
        SynthConfig {
            enable_reverb_and_chorus: false,
            gain: 1.0,
        },
        44_100,
        events,
        1_000,
    )
    .expect("dry");
    assert!(
        wet.iter()
            .zip(&dry)
            .any(|(a, b)| a.to_bits() != b.to_bits()),
        "reverb on vs off must change the A/B render"
    );
}
