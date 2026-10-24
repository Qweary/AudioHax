//! S125 lane 05 (Test Engineer): rows the PR-1 SoundFont pre-walk tests do not reach.
//!
//! The existing tests patch ONE size field of the shipped font, so they exercise the walk only on
//! the shipped font's layout. A walk that drifts from `rustysynth 1.3.6`'s reading order still
//! passes them, and then the odd-`smpl` overflow (or a 4 GiB pre-read allocation) reaches
//! `rustysynth` again. These tests build small SoundFonts from bytes and pin:
//!
//! * every chunk whose `rustysynth` reader allocates its size before reading (all nine `INFO`
//!   strings, `smpl`, `sm24`, `pmod`, `imod`) is refused when it claims more than the file holds;
//! * the boundary: a chunk that EXACTLY fits is not refused; one byte more is;
//! * the walk follows `rustysynth`'s quirks rather than the RIFF spec: `ifil` consumes 4 bytes
//!   whatever its declared size, and a list whose last chunk overruns the list's declared end is
//!   followed by the next list at the overrun position, not at the declared end;
//! * an odd `sm24` is NOT refused (its reader allocates bytes, not samples: no overflow).
//!
//! Entry point: `synth_sink::render_events_to_stereo`, which loads the font first. Nothing is
//! written to disk.
#![cfg(feature = "synth")]

use audiohax::synth_sink::{render_events_to_stereo, SoundFontSource, SynthConfig};

fn chunk(id: &[u8; 4], declared: u32, data: &[u8]) -> Vec<u8> {
    let mut v = id.to_vec();
    v.extend_from_slice(&declared.to_le_bytes());
    v.extend_from_slice(data);
    v
}

fn sized(id: &[u8; 4], data: &[u8]) -> Vec<u8> {
    chunk(id, data.len() as u32, data)
}

/// `LIST` with the given type; `declared` = `None` uses the true size.
fn list(kind: &[u8; 4], body: &[u8], declared: Option<u32>) -> Vec<u8> {
    let mut v = b"LIST".to_vec();
    v.extend_from_slice(&declared.unwrap_or(4 + body.len() as u32).to_le_bytes());
    v.extend_from_slice(kind);
    v.extend_from_slice(body);
    v
}

fn riff(lists: &[Vec<u8>]) -> Vec<u8> {
    let body: Vec<u8> = lists.concat();
    let mut v = b"RIFF".to_vec();
    v.extend_from_slice(&(4 + body.len() as u32).to_le_bytes());
    v.extend_from_slice(b"sfbk");
    v.extend(body);
    v
}

const IFIL: [u8; 4] = [2, 0, 1, 0];

fn info_ok() -> Vec<u8> {
    list(b"INFO", &sized(b"ifil", &IFIL), None)
}

fn sdta_even() -> Vec<u8> {
    list(b"sdta", &sized(b"smpl", &[0u8; 64]), None)
}

/// Load the font; return the error text ("" if it loaded).
fn load(font: &[u8]) -> String {
    match render_events_to_stereo(
        SoundFontSource::Bytes(font),
        SynthConfig::default(),
        44_100,
        vec![],
        0,
    ) {
        Ok(_) => String::new(),
        Err(e) => e.to_string(),
    }
}

fn assert_refused(font: &[u8], what: &str) {
    let e = load(font);
    assert!(e.contains("this SoundFont was refused"), "{what}: {e}");
}

fn assert_not_refused(font: &[u8], what: &str) {
    let e = load(font);
    assert!(!e.contains("refused"), "{what}: the walk refused it: {e}");
}

#[test]
fn every_chunk_that_allocates_before_reading_is_refused_past_the_end() {
    const HUGE: u32 = 0xFFFF_FFF0;
    for id in [
        b"isng", b"INAM", b"irom", b"ICRD", b"IENG", b"IPRD", b"ICOP", b"ICMT", b"ISFT",
    ] {
        let mut body = sized(b"ifil", &IFIL);
        body.extend(chunk(id, HUGE, b"x\0"));
        let f = riff(&[list(b"INFO", &body, Some(HUGE))]);
        let e = load(&f);
        let name = std::str::from_utf8(id).expect("ascii");
        assert!(
            e.contains(&format!("its '{name}' chunk claims {HUGE} bytes")),
            "{name}: {e}"
        );
    }
    for id in [b"smpl", b"sm24"] {
        let f = riff(&[
            info_ok(),
            list(b"sdta", &chunk(id, HUGE, &[0; 8]), Some(HUGE)),
        ]);
        assert_refused(&f, std::str::from_utf8(id).expect("ascii"));
    }
    for id in [b"pmod", b"imod"] {
        let f = riff(&[
            info_ok(),
            sdta_even(),
            list(b"pdta", &chunk(id, HUGE, &[0; 10]), Some(HUGE)),
        ]);
        assert_refused(&f, std::str::from_utf8(id).expect("ascii"));
    }
}

/// `size == bytes left` is read by `rustysynth` without over-allocation: not refused. One more
/// byte than is left: refused. (Pins `left` and the strict `>`.)
#[test]
fn a_chunk_that_exactly_fits_is_not_refused_and_one_byte_more_is() {
    for (declared, refused) in [(6u32, false), (7, true)] {
        let mut body = sized(b"ifil", &IFIL);
        body.extend(chunk(b"INAM", declared, b"name\0\0")); // 6 bytes really follow
        let f = riff(&[list(b"INFO", &body, Some(0xFFFF))]);
        if refused {
            assert_refused(&f, "INAM 7 of 6");
        } else {
            assert_not_refused(&f, "INAM 6 of 6");
        }
    }
}

#[test]
fn an_odd_smpl_is_refused_and_an_even_one_or_an_odd_sm24_is_not() {
    let odd = riff(&[info_ok(), list(b"sdta", &sized(b"smpl", &[0u8; 63]), None)]);
    let e = load(&odd);
    assert!(e.contains("('smpl') has an odd size (63 bytes)"), "{e}");
    assert_not_refused(&riff(&[info_ok(), sdta_even()]), "even smpl");
    let mut body = sized(b"smpl", &[0u8; 64]);
    body.extend(sized(b"sm24", &[0u8; 33]));
    assert_not_refused(&riff(&[info_ok(), list(b"sdta", &body, None)]), "odd sm24");
}

/// `ifil` declared as 100 bytes: `rustysynth` reads 4 (`SoundFontVersion::new`) and takes the
/// next chunk header from there. The walk must do the same, so a 4 GiB `INAM` hidden at that
/// point is refused. A walk that skipped the declared 100 bytes would land in the zero padding,
/// hand the font to `rustysynth`, and `rustysynth` would ask for 4 GiB.
#[test]
fn the_walk_reads_ifil_as_rustysynth_does() {
    let mut body = chunk(b"ifil", 100, &IFIL);
    body.extend(chunk(b"INAM", 0xFFFF_FFF0, b""));
    body.extend(vec![0u8; 120]);
    let f = riff(&[list(b"INFO", &body, Some(0xFFFF))]);
    let e = load(&f);
    assert!(
        e.contains("its 'INAM' chunk claims 4294967280 bytes"),
        "{e}"
    );
}

/// The `INFO` list declares an end (10) that its only chunk (`ifil`, 12 bytes with header)
/// overruns. `rustysynth` stops after the chunk and reads the `sdta` list from where it is (the
/// overrun position), so the odd `smpl` there MUST be refused. A walk that jumped to the list's
/// declared end would read garbage there, give up, and let the overflow through.
#[test]
fn the_next_list_is_found_where_rustysynth_finds_it_after_an_overrun() {
    let info = list(b"INFO", &sized(b"ifil", &IFIL), Some(10));
    let f = riff(&[info, list(b"sdta", &sized(b"smpl", &[0u8; 63]), None)]);
    let e = load(&f);
    assert!(e.contains("('smpl') has an odd size"), "{e}");
}
