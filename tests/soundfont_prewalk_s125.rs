//! S125 ITEM 1b, `s125-SH5` PR-1 (lead rulings L-13, L-61): the byte-neutral RIFF pre-walk in
//! `synth_sink::load_soundfont`, run before `rustysynth 1.3.6` sees a SoundFont.
//!
//! Fixtures are built from bytes: the shipped SoundFont (`assets/soundfonts/default.sf2`, the
//! file the binary embeds) with ONE chunk-size field patched. Files are written only under
//! `CARGO_TARGET_TMPDIR`.
//!
//! * An ODD `smpl` size is refused (`rustysynth` would write one byte past its allocation).
//! * A chunk whose reader allocates its declared size BEFORE reading (`smpl`, `sm24`, an `INFO`
//!   string, `pmod`/`imod`) is refused when that size is larger than the rest of the file (a tiny
//!   file could otherwise ask for 4 GiB, and an allocation failure aborts the process).
//! * Everything else reaches `rustysynth` unchanged: the shipped font renders byte-identically
//!   through `Bytes`, `Path` and `Bundled`.
#![cfg(all(feature = "pure-analysis", feature = "synth"))]

use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::process::Command;

use audiohax::cli::{PipelineArgs, Toggle};
use audiohax::mapping_loader::{load_mappings, MappingTable};
use audiohax::render::{render, ImageInput, RenderError, RenderOutput, RenderRequest};
use audiohax::synth_sink::SoundFontSource;
use image::{ImageFormat, Rgb, RgbImage};

fn manifest(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(rel)
}

fn shipped_font() -> Vec<u8> {
    std::fs::read(manifest("assets/soundfonts/default.sf2")).expect("the shipped SoundFont")
}

fn mappings() -> MappingTable {
    load_mappings(manifest("assets/mappings.json").to_str().expect("utf-8")).expect("mappings")
}

fn picture() -> Vec<u8> {
    let img = RgbImage::from_fn(64, 48, |x, y| {
        Rgb([
            (x * 4) as u8,
            (y * 5) as u8,
            if (x / 8 + y / 8) % 2 == 0 { 220 } else { 30 },
        ])
    });
    let mut out = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgb8(img)
        .write_to(&mut out, ImageFormat::Png)
        .expect("encode");
    out.into_inner()
}

fn request() -> RenderRequest {
    RenderRequest {
        seed: Some(7),
        loop_cue: false,
        expression: Toggle::Off,
        v2: None,
        pipeline: PipelineArgs {
            steps: 4,
            ..PipelineArgs::default()
        },
        reverb: Toggle::On,
        gain: 1.0,
        event_dump: None,
    }
}

fn render_with(font: SoundFontSource<'_>) -> Result<RenderOutput, RenderError> {
    let pic = picture();
    render(
        &request(),
        ImageInput::Bytes(&pic),
        &mappings(),
        font,
        &mut |_| Ok(()),
    )
}

/// Byte offset of the first sub-chunk `id` inside the `LIST` of type `list`.
fn sub_chunk(font: &[u8], list: &[u8; 4], id: &[u8; 4]) -> usize {
    let start = font
        .windows(4)
        .position(|w| w == list)
        .unwrap_or_else(|| panic!("no {:?} list", std::str::from_utf8(list)));
    start
        + font[start..]
            .windows(4)
            .position(|w| w == id)
            .unwrap_or_else(|| panic!("no {:?} chunk", std::str::from_utf8(id)))
}

fn patched(id_list: &[u8; 4], id: &[u8; 4], size: impl Fn(u32) -> u32) -> Vec<u8> {
    let mut f = shipped_font();
    let at = sub_chunk(&f, id_list, id);
    let old = u32::from_le_bytes([f[at + 4], f[at + 5], f[at + 6], f[at + 7]]);
    f[at + 4..at + 8].copy_from_slice(&size(old).to_le_bytes());
    f
}

fn synth_err(r: Result<RenderOutput, RenderError>) -> String {
    match r {
        Ok(_) => panic!("the SoundFont was accepted"),
        Err(RenderError::Synth(e)) => e.to_string(),
        Err(other) => panic!("expected a SoundFont error, got {other:?}"),
    }
}

#[test]
fn the_shipped_font_passes_the_walk_and_renders_byte_identically() {
    let bundled = render_with(SoundFontSource::Bundled).expect("bundled");
    let bytes = shipped_font();
    let from_bytes = render_with(SoundFontSource::Bytes(&bytes)).expect("bytes");
    let from_path = render_with(SoundFontSource::Path(&manifest(
        "assets/soundfonts/default.sf2",
    )))
    .expect("path");
    for (name, out) in [("bytes", &from_bytes), ("path", &from_path)] {
        assert_eq!(out.sample_rate, bundled.sample_rate, "{name}");
        assert_eq!(out.interleaved, bundled.interleaved, "{name}");
        assert_eq!(out.events, bundled.events, "{name}");
    }
}

#[test]
fn an_odd_sample_chunk_is_refused_before_rustysynth() {
    let f = patched(b"sdta", b"smpl", |s| s | 1);
    let e = synth_err(render_with(SoundFontSource::Bytes(&f)));
    assert!(e.contains("this SoundFont was refused"), "{e}");
    assert!(e.contains("('smpl') has an odd size"), "{e}");
}

#[test]
fn chunks_that_claim_more_than_the_file_are_refused_before_rustysynth() {
    for (list, id) in [
        (b"sdta", b"smpl"),
        (b"INFO", b"INAM"),
        (b"pdta", b"pmod"),
        (b"pdta", b"imod"),
    ] {
        let f = patched(list, id, |_| 0xFFFF_FFF0);
        let e = synth_err(render_with(SoundFontSource::Bytes(&f)));
        let name = std::str::from_utf8(id).expect("ascii");
        assert!(
            e.contains(&format!(
                "its '{name}' chunk claims 4294967280 bytes but the file ends before them"
            )),
            "{name}: {e}"
        );
    }
}

#[test]
fn the_path_arm_is_walked_too_and_names_the_file() {
    let d = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("soundfont-prewalk-s125-{}", std::process::id()));
    std::fs::create_dir_all(&d).expect("dir");
    let p = d.join("odd.sf2");
    std::fs::write(&p, patched(b"sdta", b"smpl", |s| s | 1)).expect("write");
    let e = synth_err(render_with(SoundFontSource::Path(&p)));
    assert!(
        e.contains(&format!("the SoundFont {} was refused", p.display())),
        "{e}"
    );

    // Through the CLI: exit 1, no panic, no WAV.
    let wav = d.join("x.wav");
    let png = d.join("x.png");
    std::fs::write(&png, picture()).expect("write");
    let out = Command::new(env!("CARGO_BIN_EXE_audiohax"))
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .args(["render"])
        .arg(&png)
        .args(["--seed", "1", "--steps", "4", "--soundfont"])
        .arg(&p)
        .arg("--wav")
        .arg(&wav)
        .output()
        .expect("spawn");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(1), "{stderr}");
    assert!(stderr.contains("has an odd size"), "{stderr}");
    assert!(!stderr.contains("panicked"), "{stderr}");
    assert!(!wav.exists());
    let _ = std::fs::remove_dir_all(&d);
}

/// What the walk does NOT refuse keeps rustysynth's own error text (byte-neutral for errors): a
/// file that is not a SoundFont at all.
#[test]
fn a_non_soundfont_keeps_rustysynths_own_error() {
    let e = synth_err(render_with(SoundFontSource::Bytes(
        b"not a soundfont at all",
    )));
    assert!(
        e.contains("soundfont buffer parse failed: the RIFF chunk was not found"),
        "{e}"
    );
    assert!(!e.contains("refused"), "{e}");
}

// ── ROUND 2: `s125-SH6` SH6-1 (non-regular paths), SH6-2/SH6-3 (`pdta` indices) ──────────────

/// SoundFont builders, ported from SH6's `tools/gen_sf2.py` (same layout, same numbers).
mod sf2 {
    fn ck(id: &[u8; 4], data: &[u8]) -> Vec<u8> {
        let mut v = id.to_vec();
        v.extend_from_slice(&(data.len() as u32).to_le_bytes());
        v.extend_from_slice(data);
        v
    }
    fn lst(kind: &[u8; 4], body: &[u8]) -> Vec<u8> {
        let mut v = b"LIST".to_vec();
        v.extend_from_slice(&((body.len() + 4) as u32).to_le_bytes());
        v.extend_from_slice(kind);
        v.extend_from_slice(body);
        v
    }
    fn hh(a: u16, b: u16) -> Vec<u8> {
        let mut v = a.to_le_bytes().to_vec();
        v.extend_from_slice(&b.to_le_bytes());
        v
    }
    fn name20(n: &[u8]) -> Vec<u8> {
        let mut v = n.to_vec();
        v.resize(20, 0);
        v
    }
    fn phdr(name: &[u8], bag: u16) -> Vec<u8> {
        let mut v = name20(name);
        for x in [0u16, 0, bag] {
            v.extend_from_slice(&x.to_le_bytes());
        }
        v.extend_from_slice(&[0u8; 12]);
        v
    }
    fn inst(name: &[u8], bag: u16) -> Vec<u8> {
        let mut v = name20(name);
        v.extend_from_slice(&bag.to_le_bytes());
        v
    }
    fn shdr(name: &[u8], start: i32, end: i32, sl: i32, el: i32) -> Vec<u8> {
        let mut v = name20(name);
        for x in [start, end, sl, el, 44100] {
            v.extend_from_slice(&x.to_le_bytes());
        }
        v.extend_from_slice(&[60u8, 0]);
        v.extend_from_slice(&0u16.to_le_bytes());
        v.extend_from_slice(&1u16.to_le_bytes());
        v
    }
    /// One preset → one instrument → one sample; `ibag`, `igen` and the EOI bag overridable.
    pub fn pdta(ibag: Option<Vec<u8>>, igen: Option<Vec<u8>>, eoi_bag: u16) -> Vec<u8> {
        let mut v = ck(b"phdr", &[phdr(b"P", 0), phdr(b"EOP", 1)].concat());
        v.extend(ck(b"pbag", &[hh(0, 0), hh(1, 0)].concat()));
        v.extend(ck(b"pmod", &[0u8; 10]));
        v.extend(ck(b"pgen", &[hh(41, 0), hh(0, 0)].concat()));
        v.extend(ck(
            b"inst",
            &[inst(b"I", 0), inst(b"EOI", eoi_bag)].concat(),
        ));
        v.extend(ck(
            b"ibag",
            &ibag.unwrap_or_else(|| [hh(0, 0), hh(1, 0)].concat()),
        ));
        v.extend(ck(b"imod", &[0u8; 10]));
        v.extend(ck(
            b"igen",
            &igen.unwrap_or_else(|| [hh(53, 0), hh(0, 0)].concat()),
        ));
        v.extend(ck(
            b"shdr",
            &[shdr(b"S", 0, 60, 8, 50), shdr(b"EOS", 0, 0, 0, 0)].concat(),
        ));
        v
    }
    /// A whole font from its `sdta` and `pdta` bodies.
    pub fn font(sdta: &[u8], pdta: &[u8]) -> Vec<u8> {
        let info = [ck(b"ifil", &hh(2, 1)), ck(b"INAM", b"sh6\0")].concat();
        let mut body = b"sfbk".to_vec();
        body.extend(lst(b"INFO", &info));
        body.extend(lst(b"sdta", sdta));
        body.extend(lst(b"pdta", pdta));
        let mut v = b"RIFF".to_vec();
        v.extend_from_slice(&(body.len() as u32).to_le_bytes());
        v.extend(body);
        v
    }
    pub fn smpl(bytes: usize) -> Vec<u8> {
        ck(b"smpl", &vec![0u8; bytes])
    }
    /// S00: SH6's minimal valid font.
    pub fn s00() -> Vec<u8> {
        font(&smpl(128), &pdta(None, None, 1))
    }
    /// S02: an `smpl` of size 1 (plus its RIFF pad byte).
    pub fn s02() -> Vec<u8> {
        font(&[smpl(1), vec![0]].concat(), &pdta(None, None, 1))
    }
    /// S04: header, INFO, and an `sdta` whose `smpl` claims 4,294,967,280 bytes; then the file ends.
    pub fn s04() -> Vec<u8> {
        let mut v = b"RIFF".to_vec();
        v.extend_from_slice(&0u32.to_le_bytes());
        v.extend_from_slice(b"sfbk");
        v.extend(lst(b"INFO", &ck(b"ifil", &hh(2, 1))));
        v.extend_from_slice(b"LIST");
        v.extend_from_slice(&0xFFFF_FFF8u32.to_le_bytes());
        v.extend_from_slice(b"sdta");
        v.extend_from_slice(b"smpl");
        v.extend_from_slice(&0xFFFF_FFF0u32.to_le_bytes());
        v
    }
    /// S09–S11: the zone bomb (alternating bag indices 0, 65535 over 65,536 generators).
    pub fn zone_bomb(zones: usize) -> Vec<u8> {
        let mut igen = Vec::new();
        for _ in 0..65536 {
            igen.extend(hh(53, 0));
        }
        igen.extend(hh(0, 0));
        let mut ibag = Vec::new();
        for k in 0..zones {
            ibag.extend(hh(if k % 2 == 0 { 0 } else { 65535 }, 0));
        }
        ibag.extend(hh(0, 0));
        font(&smpl(128), &pdta(Some(ibag), Some(igen), zones as u16))
    }
    /// S12: an `ibag` index of 60,000 against a one-entry `igen`.
    pub fn s12() -> Vec<u8> {
        font(
            &smpl(128),
            &pdta(Some([hh(0, 0), hh(60000, 0)].concat()), None, 1),
        )
    }
    /// An `inst` range past the `ibag` zone list (the SH6-3 `Instrument::new` slice site).
    pub fn inst_past_zones() -> Vec<u8> {
        font(&smpl(128), &pdta(None, None, 9))
    }
}

#[test]
fn sh6_minimal_valid_font_still_loads_and_renders() {
    render_with(SoundFontSource::Bytes(&sf2::s00())).expect("S00 renders");
}

/// SH6-2 / SH6-3: S09–S12 and an `inst` range past its zones are refused before rustysynth,
/// through `Bytes` and `Path`. (SH6's < 50 ms / < 64 MiB condition is for the CLI process and is
/// measured in the lane report; here the bound only catches a walk that went wrong.)
#[test]
fn sh6_pdta_index_shapes_are_refused_fast() {
    use std::time::{Duration, Instant};
    let rows: Vec<(&str, Vec<u8>, &str)> = vec![
        ("S09", sf2::zone_bomb(2000), "('ibag') would copy"),
        ("S10", sf2::zone_bomb(8000), "('ibag') would copy"),
        ("S11", sf2::zone_bomb(40000), "('ibag') would copy"),
        (
            "S12",
            sf2::s12(),
            "('ibag') points to generator 60000, but its generator list ('igen') has only 1",
        ),
        (
            "inst",
            sf2::inst_past_zones(),
            "its 'inst' list points to zone 9, but its zone list ('ibag') has only 1",
        ),
    ];
    for (name, bytes, want) in rows {
        let e = synth_err(render_with(SoundFontSource::Bytes(&bytes)));
        let d = PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
        let p = d.join(format!("sh6-{name}-{}.sf2", std::process::id()));
        std::fs::write(&p, &bytes).expect("write");
        let t1 = Instant::now();
        let e2 = synth_err(render_with(SoundFontSource::Path(&p)));
        let took = t1.elapsed();
        let _ = std::fs::remove_file(&p);
        assert!(e.contains(want), "{name}: {e}");
        assert!(e2.contains(want), "{name}: {e2}");
        assert!(e.contains("this SoundFont was refused"), "{name}: {e}");
        // Includes the picture analysis of a 64×48 render request; the walk itself is a small
        // part of it. The pre-S125 binary needed 1.6–5.6 s and up to 4 GiB here.
        assert!(took < Duration::from_secs(5), "{name}: {took:?}");
    }
}

/// SH6-1: a non-regular `--soundfont` path (a FIFO) is walked too. S02 (the one-byte overflow,
/// a SIGSEGV before) and S04 (a 4 GiB claim, a SIGABRT before) are refused; the shipped font
/// through a FIFO renders byte-identically to the regular file.
#[cfg(unix)]
#[test]
fn sh6_1_a_fifo_soundfont_is_walked() {
    use std::io::Write;
    let d = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("soundfont-fifo-s125-{}", std::process::id()));
    std::fs::create_dir_all(&d).expect("dir");
    let through_fifo = |name: &str, bytes: Vec<u8>| -> Result<RenderOutput, RenderError> {
        let fifo = d.join(name);
        let st = Command::new("mkfifo").arg(&fifo).status().expect("mkfifo");
        assert!(st.success());
        let writer = {
            let fifo = fifo.clone();
            std::thread::spawn(move || {
                if let Ok(mut f) = std::fs::OpenOptions::new().write(true).open(&fifo) {
                    let _ = f.write_all(&bytes);
                }
            })
        };
        let r = render_with(SoundFontSource::Path(&fifo));
        // If the walk refused before reading everything, the reader is closed by now, so the
        // writer's next write fails (EPIPE; Rust ignores SIGPIPE) and the thread ends.
        let _ = writer.join();
        r
    };
    let e = synth_err(through_fifo("s02.sf2", sf2::s02()));
    assert!(e.contains("('smpl') has an odd size (1 bytes)"), "{e}");
    let e = synth_err(through_fifo("s04.sf2", sf2::s04()));
    assert!(
        e.contains("its 'smpl' chunk claims 4294967280 bytes but the file ends before them"),
        "{e}"
    );
    let e = synth_err(through_fifo("s11.sf2", sf2::zone_bomb(40000)));
    assert!(e.contains("would copy"), "{e}");

    let regular = render_with(SoundFontSource::Path(&manifest(
        "assets/soundfonts/default.sf2",
    )))
    .expect("regular");
    let piped = through_fifo("shipped.sf2", shipped_font()).expect("the shipped font via a FIFO");
    assert_eq!(piped.interleaved, regular.interleaved);
    assert_eq!(piped.events, regular.events);
    let _ = std::fs::remove_dir_all(&d);
}

/// SH6-2's rule bounds what `rustysynth` COPIES, not the order of the indices: the shipped font
/// with its first `pbag` generator index set to 65535 (lane 05's `pbag_gen.sf2` shape: one step
/// goes down, so that zone copies nothing) still loads and renders byte-identically, as it did
/// before; the shipped font with its second `phdr` bag index set to 65535 (lane 05's
/// `phdr_zone.sf2`, a `preset.rs:38` panic before) is refused.
#[test]
fn sh6_harmless_index_quirks_still_load_and_panicking_ones_are_refused() {
    let shipped = shipped_font();
    let pbag = sub_chunk(&shipped, b"pdta", b"pbag");
    let mut quirk = shipped.clone();
    quirk[pbag + 8..pbag + 10].copy_from_slice(&65535u16.to_le_bytes());
    let normal = render_with(SoundFontSource::Bytes(&shipped)).expect("shipped");
    let quirky = render_with(SoundFontSource::Bytes(&quirk)).expect("the quirk still loads");
    assert_eq!(quirky.interleaved, normal.interleaved);
    assert_eq!(quirky.events, normal.events);

    let phdr = sub_chunk(&shipped, b"pdta", b"phdr");
    let mut bad = shipped.clone();
    // Record 1 (the second), bag index at offset 24 of the 38-byte record.
    let at = phdr + 8 + 38 + 24;
    bad[at..at + 2].copy_from_slice(&65535u16.to_le_bytes());
    let e = synth_err(render_with(SoundFontSource::Bytes(&bad)));
    assert!(e.contains("its 'phdr' list points to zone 65535"), "{e}");
}

/// SH6's condition "S09–S12 end rc 1 in < 50 ms": the walk alone (no picture analysis), through
/// `synth_sink::render_events_to_stereo`, which loads the font first.
#[test]
fn sh6_pdta_refusals_are_fast_in_the_walk_itself() {
    use audiohax::synth_sink::{render_events_to_stereo, SynthConfig};
    use std::time::{Duration, Instant};
    for (name, bytes) in [
        ("S09", sf2::zone_bomb(2000)),
        ("S10", sf2::zone_bomb(8000)),
        ("S11", sf2::zone_bomb(40000)),
        ("S12", sf2::s12()),
    ] {
        let t0 = Instant::now();
        let r = render_events_to_stereo(
            SoundFontSource::Bytes(&bytes),
            SynthConfig::default(),
            44_100,
            vec![],
            0,
        );
        let took = t0.elapsed();
        let e = r.map(|_| ()).expect_err("refused").to_string();
        assert!(e.contains("this SoundFont was refused"), "{name}: {e}");
        assert!(took < Duration::from_millis(50), "{name}: {took:?}");
    }
}
