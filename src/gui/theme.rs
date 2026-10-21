//! The operator palette (UI `s123-UI` § 10.2; operator ruling D-41, `s123-00-OPERATOR-RULINGS.md`
//! § A.1: "frost blue and ethereal purple (without things being too blinding), and a dark mode is
//! also necessary").
//!
//! Dark is the default; light is offered with the same two hues, deepened. Every colour the
//! window paints comes from a [`Palette`] token (UI A8: no stock egui colour, no one-off hex in the
//! drawing code). The hex values are the designer's; the contrast figures in the tests are
//! recomputed here from those hex values with the WCAG 2.1 formula.

use eframe::egui::{
    self, Color32, CornerRadius, FontFamily, FontId, Margin, Stroke, TextStyle, Visuals,
};

/// One mode's tokens.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Palette {
    /// `true` for the dark palette.
    pub dark: bool,
    /// `color.bg`: window and stage ground.
    pub bg: Color32,
    /// `color.surface`: panels.
    pub surface: Color32,
    /// `color.raised`: cards, modals, idle widget fill.
    pub raised: Color32,
    /// One step above `raised`: hover / active fill (calm, never an accent flash).
    pub raised_hover: Color32,
    /// `color.text`: body text.
    pub text: Color32,
    /// `color.text-muted`: secondary labels.
    pub text_muted: Color32,
    /// `color.text-faint`: hints and units (on `bg` only).
    pub text_faint: Color32,
    /// `color.control-border`: input outlines, slider tracks.
    pub control_border: Color32,
    /// `color.accent` (frost blue): primary action, scan bar, selection.
    pub accent: Color32,
    /// Ink drawn ON the accent (a label on the primary button).
    pub on_accent: Color32,
    /// `color.accent-2` (ethereal purple): reserved (S124 modified marker, A/B).
    pub accent_2: Color32,
    /// `color.focus`: the 2 px focus ring.
    pub focus: Color32,
    /// `color.warn`: dependency notices.
    pub warn: Color32,
    /// `color.error`: error headlines.
    pub error: Color32,
}

/// Parse `#RRGGBB` at compile time.
const fn hex(rgb: u32) -> Color32 {
    Color32::from_rgb((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8)
}

impl Palette {
    /// The DEFAULT palette (UI § 10.2 dark table).
    pub const DARK: Palette = Palette {
        dark: true,
        bg: hex(0x13121A),
        surface: hex(0x1C1B26),
        raised: hex(0x262532),
        raised_hover: hex(0x2F2E3D),
        text: hex(0xDADCE6),
        text_muted: hex(0x9D9AB0),
        text_faint: hex(0x7E7B92),
        control_border: hex(0x6E6A82),
        accent: hex(0x7CC4E8),
        on_accent: hex(0x13121A),
        accent_2: hex(0xB6A6E6),
        focus: hex(0xC9B6F0),
        warn: hex(0xE8C06A),
        error: hex(0xE88A8A),
    };

    /// The optional light palette (UI § 10.2 light table). The designer's table gives no
    /// `text-faint` or hover step for light mode; `text_faint` reuses `text-muted` (AA on every
    /// light surface) and `raised_hover` is one step darker than `bg` (flagged in the report).
    pub const LIGHT: Palette = Palette {
        dark: false,
        bg: hex(0xF4F3F8),
        surface: hex(0xFBFAFD),
        raised: hex(0xFFFFFF),
        raised_hover: hex(0xE9E7F1),
        text: hex(0x20202B),
        text_muted: hex(0x55536A),
        text_faint: hex(0x55536A),
        control_border: hex(0xB7B3C6),
        accent: hex(0x2E7CB0),
        on_accent: hex(0xFFFFFF),
        accent_2: hex(0x6B52B0),
        focus: hex(0x5B86C9),
        warn: hex(0x9A6B12),
        error: hex(0xB23B3B),
    };

    /// The palette for a mode.
    pub fn for_mode(dark: bool) -> Palette {
        if dark {
            Palette::DARK
        } else {
            Palette::LIGHT
        }
    }

    /// `color` with alpha `a` (0..=1), unmultiplied.
    pub fn with_alpha(color: Color32, a: f32) -> Color32 {
        Color32::from_rgba_unmultiplied(
            color.r(),
            color.g(),
            color.b(),
            (a.clamp(0.0, 1.0) * 255.0).round() as u8,
        )
    }

    /// The egui `Visuals` for this palette (the field mapping of UI § 10.2).
    pub fn visuals(&self) -> Visuals {
        let mut v = if self.dark {
            Visuals::dark()
        } else {
            Visuals::light()
        };
        v.override_text_color = Some(self.text);
        v.panel_fill = self.surface;
        v.window_fill = self.raised;
        v.window_stroke = Stroke::new(1.0, self.control_border);
        v.extreme_bg_color = self.bg;
        v.faint_bg_color = self.raised;
        v.text_edit_bg_color = Some(self.bg);
        v.code_bg_color = self.raised;
        v.hyperlink_color = self.accent;
        v.warn_fg_color = self.warn;
        v.error_fg_color = self.error;
        v.selection.bg_fill = Palette::with_alpha(self.accent, 0.35);
        // The focus ring: a 2 px lavender stroke, distinct from the blue primary.
        v.selection.stroke = Stroke::new(2.0, self.focus);
        v.window_corner_radius = CornerRadius::same(10);
        v.menu_corner_radius = CornerRadius::same(6);
        v.slider_trailing_fill = true;

        let r6 = CornerRadius::same(6);
        let w = &mut v.widgets;
        w.noninteractive.bg_fill = self.surface;
        w.noninteractive.weak_bg_fill = self.surface;
        w.noninteractive.bg_stroke =
            Stroke::new(1.0, Palette::with_alpha(self.control_border, 0.5));
        w.noninteractive.fg_stroke = Stroke::new(1.0, self.text_muted);
        w.noninteractive.corner_radius = r6;

        w.inactive.bg_fill = self.raised;
        w.inactive.weak_bg_fill = self.raised;
        w.inactive.bg_stroke = Stroke::new(1.0, self.control_border);
        w.inactive.fg_stroke = Stroke::new(1.0, self.text);
        w.inactive.corner_radius = r6;

        w.hovered.bg_fill = self.raised_hover;
        w.hovered.weak_bg_fill = self.raised_hover;
        w.hovered.bg_stroke = Stroke::new(1.0, self.accent);
        w.hovered.fg_stroke = Stroke::new(1.5, self.text);
        w.hovered.corner_radius = r6;

        w.active.bg_fill = self.raised_hover;
        w.active.weak_bg_fill = self.raised_hover;
        w.active.bg_stroke = Stroke::new(2.0, self.focus);
        w.active.fg_stroke = Stroke::new(2.0, self.text);
        w.active.corner_radius = r6;

        w.open.bg_fill = self.raised_hover;
        w.open.weak_bg_fill = self.raised_hover;
        w.open.bg_stroke = Stroke::new(1.0, self.control_border);
        w.open.fg_stroke = Stroke::new(1.0, self.text);
        w.open.corner_radius = r6;
        v
    }

    /// Apply this palette, the type scale (`type.body` 15 / `type.label` 13 / `type.mono` 13 px)
    /// and the 4-px spacing base to `ctx`.
    pub fn apply(&self, ctx: &egui::Context) {
        ctx.set_visuals(self.visuals());
        ctx.all_styles_mut(|style| {
            style.text_styles = [
                (
                    TextStyle::Heading,
                    FontId::new(20.0, FontFamily::Proportional),
                ),
                (TextStyle::Body, FontId::new(15.0, FontFamily::Proportional)),
                (
                    TextStyle::Button,
                    FontId::new(15.0, FontFamily::Proportional),
                ),
                (
                    TextStyle::Small,
                    FontId::new(13.0, FontFamily::Proportional),
                ),
                (
                    TextStyle::Monospace,
                    FontId::new(13.0, FontFamily::Monospace),
                ),
            ]
            .into();
            style.spacing.item_spacing = egui::vec2(8.0, 8.0);
            style.spacing.button_padding = egui::vec2(12.0, 6.0);
            style.spacing.window_margin = Margin::same(16);
            style.spacing.menu_margin = Margin::same(8);
            style.spacing.interact_size.y = 28.0;
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// WCAG 2.1 relative luminance.
    fn lum(c: Color32) -> f64 {
        let ch = |v: u8| {
            let s = f64::from(v) / 255.0;
            if s <= 0.04045 {
                s / 12.92
            } else {
                ((s + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * ch(c.r()) + 0.7152 * ch(c.g()) + 0.0722 * ch(c.b())
    }

    fn contrast(a: Color32, b: Color32) -> f64 {
        let (x, y) = (lum(a), lum(b));
        let (hi, lo) = if x > y { (x, y) } else { (y, x) };
        (hi + 0.05) / (lo + 0.05)
    }

    #[test]
    fn a8_dark_contrasts_meet_aa() {
        let p = Palette::DARK;
        for ground in [p.bg, p.surface, p.raised] {
            assert!(contrast(p.text, ground) >= 4.5);
            assert!(contrast(p.text_muted, ground) >= 4.5);
            assert!(contrast(p.focus, ground) >= 3.0);
        }
        // The designer states control-border contrast on bg and surface (3.59 / 3.28); on
        // `raised` it is 2.91, which is why widget outlines sit against the panel (surface).
        for ground in [p.bg, p.surface] {
            assert!(contrast(p.control_border, ground) >= 3.0);
        }
        assert!(contrast(p.text_faint, p.bg) >= 4.5);
        assert!(
            contrast(p.on_accent, p.accent) >= 4.5,
            "primary button label"
        );
        // "Not too blinding": body text is NOT pure white on near-black.
        assert!(contrast(p.text, p.bg) < 15.0);
        assert_ne!(p.text, Color32::WHITE);
    }

    #[test]
    fn a8_light_contrasts_meet_aa() {
        let p = Palette::LIGHT;
        for ground in [p.bg, p.surface, p.raised] {
            assert!(contrast(p.text, ground) >= 4.5);
            assert!(contrast(p.text_muted, ground) >= 4.5);
            assert!(contrast(p.focus, ground) >= 3.0);
        }
        assert!(
            contrast(p.on_accent, p.accent) >= 4.5,
            "primary button label"
        );
        assert!(contrast(p.error, p.bg) >= 4.5);
        // warn is AA on the panels where the window draws it (surface 4.50, raised 4.68).
        assert!(contrast(p.warn, p.surface) >= 4.5);
        // KNOWN, flagged to the UI Designer (build report): the light table's control-border
        // #B7B3C6 measures 1.85 / 1.97 / 2.04 on bg / surface / raised, not the stated >= 3:1,
        // and warn #9A6B12 measures 4.24 on bg. The designer's hex values are kept as given.
        assert!(contrast(p.control_border, p.bg) < 3.0);
    }

    #[test]
    fn designer_figures_reproduce() {
        let p = Palette::DARK;
        // UI § 10.2 states 13.60 / 9.67 / 9.67.
        assert!((contrast(p.text, p.bg) - 13.60).abs() < 0.05);
        assert!((contrast(p.accent, p.bg) - 9.67).abs() < 0.05);
        assert!((contrast(p.on_accent, p.accent) - 9.67).abs() < 0.05);
    }

    #[test]
    fn dark_is_the_default() {
        assert!(Palette::for_mode(true).dark);
        assert_eq!(Palette::for_mode(true), Palette::DARK);
    }
}
