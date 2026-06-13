// tests/qg_probe_band_isolation.rs
//
// QG (S7 review) INDEPENDENT PROBE — NOT part of the Pass-B spec net.
//
// Purpose: independently verify the riskiest S7 divergence — that widening
// goertzel_mag_squared to a ±0.75-bin, 5-point band-energy sum did NOT destroy
// adjacent-TONE isolation WITHIN a channel (the band-isolation unit test in
// src/modem.rs only checks adjacent CHANNELS, which are separated by a 450 Hz
// guard band; the tighter case is two tones 50 Hz / 2 bins apart in the SAME
// channel). This test renders one pure on-bin tone and confirms the band detector
// still arg-maxes onto the TRUE tone, with the immediate neighbours strictly lower.
//
// This is a PROBE, not a verdict driver: the verdict rests on Pass-B's net. If this
// probe failed it would be a strong signal the widening aliases; if it passes it
// corroborates the re-derived math (band edges 0.5 bin / 12.5 Hz apart, no overlap).
//
//   cargo test --test qg_probe_band_isolation --no-default-features

use audiohax::modem::{self, ModemParams};

/// well_separated config mirror (2 ch / 8 tones / 40 ms): 25 Hz bins, 50 Hz tone
/// spacing = 2 bins — the exact within-channel adjacency the ±0.75-bin band must
/// still resolve.
fn params() -> ModemParams {
    let mut p = ModemParams::default();
    p.channels = 2;
    p.m_tones = 8;
    p.symbol_ms = 40.0;
    p.preamble_symbols = vec![4u8];
    p
}

#[test]
fn probe_band_detector_resolves_adjacent_in_channel_tones() {
    let p = params();
    let freqs = modem::build_tone_frequencies(&p);
    let ch = 0usize;
    let nt = p.m_tones;

    // For each interior tone, render it pure and confirm the band detector's arg-max
    // over the channel's tones is the tone itself, and each immediate neighbour reads
    // strictly lower than the true tone.
    for t in 1..nt - 1 {
        let samples = render_one_tone(&p, ch, t as u8);
        assert!(!samples.is_empty());

        let mags: Vec<f32> = freqs[ch]
            .iter()
            .map(|&f| modem::goertzel_mag_squared(&samples, f, p.sample_rate))
            .collect();

        // arg-max must be the emitted tone.
        let (argmax, &peak) = mags
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .unwrap();
        assert_eq!(
            argmax, t,
            "band detector must arg-max onto the emitted tone {t}, got {argmax} \
             (mags = {mags:?}) — the ±0.75-bin widening must not alias onto a neighbour"
        );

        // Both immediate neighbours strictly below the true tone (isolation margin).
        assert!(
            mags[t - 1] < peak && mags[t + 1] < peak,
            "neighbours of tone {t} must read below it: left={}, true={peak}, right={} \
             (50 Hz / 2-bin spacing vs ±0.75-bin band ⇒ 0.5-bin dead zone, no overlap)",
            mags[t - 1],
            mags[t + 1],
        );

        // Quantitative: neighbour leakage well under the true-tone response.
        assert!(
            mags[t - 1] < 0.5 * peak && mags[t + 1] < 0.5 * peak,
            "neighbour leakage for tone {t} must stay well under the on-tone response \
             (left={:.3e}, right={:.3e}, true={peak:.3e})",
            mags[t - 1],
            mags[t + 1],
        );
    }
}

/// Render a single pure tone in `ch` at tone index `tone` across one symbol window,
/// using only public modem API (split/render mirror of the encode bins).
fn render_one_tone(p: &ModemParams, ch: usize, tone: u8) -> Vec<i16> {
    // One symbol on the target channel; other channels silent (empty).
    let mut per_channel: Vec<Vec<u8>> = vec![Vec::new(); p.channels];
    per_channel[ch] = vec![tone];
    modem::render_symbols_to_samples(&per_channel, p)
}
