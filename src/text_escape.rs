//! Escaping untrusted text for a terminal or a log line (MB-8; the security review's SH3-1,
//! SH3-2 and SH4; S125b lead ruling L-94).
//!
//! A mappings file, a file name or a JSON key can carry characters that drive a terminal (ESC,
//! the 8-bit CSI, BEL), reorder what the user reads (bidi overrides and isolates), or hide text
//! (zero-width characters, variation selectors, Hangul fillers, tag characters). Every place
//! that prints such text writes those characters as visible escapes instead.
//!
//! THE ESCAPED SET IS FROZEN IN THIS FILE. It is four disjoint classes ([`EscapeClass`]) stored
//! as range tables generated from the Unicode Character Database 17.0.0 by
//! `tools/gen_text_escape_tables.py`. Nothing on the decision path or the rendering path calls
//! `char::escape_debug`, `str::escape_debug`, `char::is_control`, `Debug` formatting or any other
//! std Unicode table, so the set a binary escapes, and how it writes each escape, do not depend
//! on the rustc that built it. (Until S125b the set WAS std's: rustc 1.99 began escaping every
//! Default_Ignorable_Code_Point, 267 more than 1.96.0, and the same source printed different
//! text depending on the compiler.) The set frozen here is exactly what rustc 1.99.1's
//! `str::escape_debug` escapes (measured over every code point, S125b).
//!
//! The tests compare the frozen set with the BUILDING toolchain's `str::escape_debug` and fail
//! only when std escapes something this set prints as written (a possible hole). A std that
//! escapes less than this set is reported, not failed.
//!
//! To change the set: change the class definitions in the generator, run it, and update the
//! counts and digest in `tests::escape_set_is_frozen`. Never hand-edit a generated table.

use std::cmp::Ordering;
use std::fmt::Write as _;

/// Why a scalar value is escaped. The four classes are disjoint; a scalar is in at most one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EscapeClass {
    /// General_Category Cc: C0, DEL and C1 (ESC, BEL, the 8-bit CSI U+009B, …).
    Control,
    /// Format characters (Cf: bidi overrides and isolates, zero-width and direction marks, the
    /// BOM, tags), non-ASCII spaces (Zs other than U+0020), line and paragraph separators (Zl,
    /// Zp) and every Default_Ignorable_Code_Point (variation selectors, the combining grapheme
    /// joiner, Hangul fillers, …).
    Invisible,
    /// Private-use code points (Co).
    PrivateUse,
    /// Code points unassigned in Unicode 17.0.0 (Cn), other than those already in `Invisible`.
    Unassigned,
}

/// An inclusive range of scalar values, `(first, last)`. Each table is sorted, non-overlapping
/// and non-adjacent, and never contains a surrogate (U+D800..=U+DFFF).
pub type ScalarRange = (u32, u32);

// BEGIN GENERATED TABLES — written by tools/gen_text_escape_tables.py from UCD 17.0.0; DO NOT EDIT BY HAND.
// Inputs (sha256): DerivedGeneralCategory.txt d62e5bab70ca74f099343f71224fa051cb1fdd61a1ab45c0488c44cfc0b6102e,
// DerivedCoreProperties.txt 24c7fed1195c482faaefd5c1e7eb821c5ee1fb6de07ecdbaa64b56a99da22c08.

/// The Unicode version the tables below were generated from.
pub const TABLES_UNICODE_VERSION: (u8, u8, u8) = (17, 0, 0);

/// [`EscapeClass::Control`]: General_Category Cc (65 code points).
#[rustfmt::skip]
pub const CONTROL: &[ScalarRange] = &[
    (0x0000, 0x001F),
    (0x007F, 0x009F),
];

/// [`EscapeClass::Invisible`]: Cf, Zs other than U+0020, Zl, Zp and every
/// Default_Ignorable_Code_Point, less Cc (4224 code points).
#[rustfmt::skip]
pub const INVISIBLE: &[ScalarRange] = &[
    (0x00A0, 0x00A0),
    (0x00AD, 0x00AD),
    (0x034F, 0x034F),
    (0x0600, 0x0605),
    (0x061C, 0x061C),
    (0x06DD, 0x06DD),
    (0x070F, 0x070F),
    (0x0890, 0x0891),
    (0x08E2, 0x08E2),
    (0x115F, 0x1160),
    (0x1680, 0x1680),
    (0x17B4, 0x17B5),
    (0x180B, 0x180F),
    (0x2000, 0x200F),
    (0x2028, 0x202F),
    (0x205F, 0x206F),
    (0x3000, 0x3000),
    (0x3164, 0x3164),
    (0xFE00, 0xFE0F),
    (0xFEFF, 0xFEFF),
    (0xFFA0, 0xFFA0),
    (0xFFF0, 0xFFFB),
    (0x110BD, 0x110BD),
    (0x110CD, 0x110CD),
    (0x13430, 0x1343F),
    (0x1BCA0, 0x1BCA3),
    (0x1D173, 0x1D17A),
    (0xE0000, 0xE0FFF),
];

/// [`EscapeClass::PrivateUse`]: Co, less Invisible (137468 code points).
#[rustfmt::skip]
pub const PRIVATE_USE: &[ScalarRange] = &[
    (0xE000, 0xF8FF),
    (0xF0000, 0xFFFFD),
    (0x100000, 0x10FFFD),
];

/// [`EscapeClass::Unassigned`]: Cn in Unicode 17.0.0, less Invisible and PrivateUse
/// (810961 code points).
#[rustfmt::skip]
pub const UNASSIGNED: &[ScalarRange] = &[
    (0x0378, 0x0379),
    (0x0380, 0x0383),
    (0x038B, 0x038B),
    (0x038D, 0x038D),
    (0x03A2, 0x03A2),
    (0x0530, 0x0530),
    (0x0557, 0x0558),
    (0x058B, 0x058C),
    (0x0590, 0x0590),
    (0x05C8, 0x05CF),
    (0x05EB, 0x05EE),
    (0x05F5, 0x05FF),
    (0x070E, 0x070E),
    (0x074B, 0x074C),
    (0x07B2, 0x07BF),
    (0x07FB, 0x07FC),
    (0x082E, 0x082F),
    (0x083F, 0x083F),
    (0x085C, 0x085D),
    (0x085F, 0x085F),
    (0x086B, 0x086F),
    (0x0892, 0x0896),
    (0x0984, 0x0984),
    (0x098D, 0x098E),
    (0x0991, 0x0992),
    (0x09A9, 0x09A9),
    (0x09B1, 0x09B1),
    (0x09B3, 0x09B5),
    (0x09BA, 0x09BB),
    (0x09C5, 0x09C6),
    (0x09C9, 0x09CA),
    (0x09CF, 0x09D6),
    (0x09D8, 0x09DB),
    (0x09DE, 0x09DE),
    (0x09E4, 0x09E5),
    (0x09FF, 0x0A00),
    (0x0A04, 0x0A04),
    (0x0A0B, 0x0A0E),
    (0x0A11, 0x0A12),
    (0x0A29, 0x0A29),
    (0x0A31, 0x0A31),
    (0x0A34, 0x0A34),
    (0x0A37, 0x0A37),
    (0x0A3A, 0x0A3B),
    (0x0A3D, 0x0A3D),
    (0x0A43, 0x0A46),
    (0x0A49, 0x0A4A),
    (0x0A4E, 0x0A50),
    (0x0A52, 0x0A58),
    (0x0A5D, 0x0A5D),
    (0x0A5F, 0x0A65),
    (0x0A77, 0x0A80),
    (0x0A84, 0x0A84),
    (0x0A8E, 0x0A8E),
    (0x0A92, 0x0A92),
    (0x0AA9, 0x0AA9),
    (0x0AB1, 0x0AB1),
    (0x0AB4, 0x0AB4),
    (0x0ABA, 0x0ABB),
    (0x0AC6, 0x0AC6),
    (0x0ACA, 0x0ACA),
    (0x0ACE, 0x0ACF),
    (0x0AD1, 0x0ADF),
    (0x0AE4, 0x0AE5),
    (0x0AF2, 0x0AF8),
    (0x0B00, 0x0B00),
    (0x0B04, 0x0B04),
    (0x0B0D, 0x0B0E),
    (0x0B11, 0x0B12),
    (0x0B29, 0x0B29),
    (0x0B31, 0x0B31),
    (0x0B34, 0x0B34),
    (0x0B3A, 0x0B3B),
    (0x0B45, 0x0B46),
    (0x0B49, 0x0B4A),
    (0x0B4E, 0x0B54),
    (0x0B58, 0x0B5B),
    (0x0B5E, 0x0B5E),
    (0x0B64, 0x0B65),
    (0x0B78, 0x0B81),
    (0x0B84, 0x0B84),
    (0x0B8B, 0x0B8D),
    (0x0B91, 0x0B91),
    (0x0B96, 0x0B98),
    (0x0B9B, 0x0B9B),
    (0x0B9D, 0x0B9D),
    (0x0BA0, 0x0BA2),
    (0x0BA5, 0x0BA7),
    (0x0BAB, 0x0BAD),
    (0x0BBA, 0x0BBD),
    (0x0BC3, 0x0BC5),
    (0x0BC9, 0x0BC9),
    (0x0BCE, 0x0BCF),
    (0x0BD1, 0x0BD6),
    (0x0BD8, 0x0BE5),
    (0x0BFB, 0x0BFF),
    (0x0C0D, 0x0C0D),
    (0x0C11, 0x0C11),
    (0x0C29, 0x0C29),
    (0x0C3A, 0x0C3B),
    (0x0C45, 0x0C45),
    (0x0C49, 0x0C49),
    (0x0C4E, 0x0C54),
    (0x0C57, 0x0C57),
    (0x0C5B, 0x0C5B),
    (0x0C5E, 0x0C5F),
    (0x0C64, 0x0C65),
    (0x0C70, 0x0C76),
    (0x0C8D, 0x0C8D),
    (0x0C91, 0x0C91),
    (0x0CA9, 0x0CA9),
    (0x0CB4, 0x0CB4),
    (0x0CBA, 0x0CBB),
    (0x0CC5, 0x0CC5),
    (0x0CC9, 0x0CC9),
    (0x0CCE, 0x0CD4),
    (0x0CD7, 0x0CDB),
    (0x0CDF, 0x0CDF),
    (0x0CE4, 0x0CE5),
    (0x0CF0, 0x0CF0),
    (0x0CF4, 0x0CFF),
    (0x0D0D, 0x0D0D),
    (0x0D11, 0x0D11),
    (0x0D45, 0x0D45),
    (0x0D49, 0x0D49),
    (0x0D50, 0x0D53),
    (0x0D64, 0x0D65),
    (0x0D80, 0x0D80),
    (0x0D84, 0x0D84),
    (0x0D97, 0x0D99),
    (0x0DB2, 0x0DB2),
    (0x0DBC, 0x0DBC),
    (0x0DBE, 0x0DBF),
    (0x0DC7, 0x0DC9),
    (0x0DCB, 0x0DCE),
    (0x0DD5, 0x0DD5),
    (0x0DD7, 0x0DD7),
    (0x0DE0, 0x0DE5),
    (0x0DF0, 0x0DF1),
    (0x0DF5, 0x0E00),
    (0x0E3B, 0x0E3E),
    (0x0E5C, 0x0E80),
    (0x0E83, 0x0E83),
    (0x0E85, 0x0E85),
    (0x0E8B, 0x0E8B),
    (0x0EA4, 0x0EA4),
    (0x0EA6, 0x0EA6),
    (0x0EBE, 0x0EBF),
    (0x0EC5, 0x0EC5),
    (0x0EC7, 0x0EC7),
    (0x0ECF, 0x0ECF),
    (0x0EDA, 0x0EDB),
    (0x0EE0, 0x0EFF),
    (0x0F48, 0x0F48),
    (0x0F6D, 0x0F70),
    (0x0F98, 0x0F98),
    (0x0FBD, 0x0FBD),
    (0x0FCD, 0x0FCD),
    (0x0FDB, 0x0FFF),
    (0x10C6, 0x10C6),
    (0x10C8, 0x10CC),
    (0x10CE, 0x10CF),
    (0x1249, 0x1249),
    (0x124E, 0x124F),
    (0x1257, 0x1257),
    (0x1259, 0x1259),
    (0x125E, 0x125F),
    (0x1289, 0x1289),
    (0x128E, 0x128F),
    (0x12B1, 0x12B1),
    (0x12B6, 0x12B7),
    (0x12BF, 0x12BF),
    (0x12C1, 0x12C1),
    (0x12C6, 0x12C7),
    (0x12D7, 0x12D7),
    (0x1311, 0x1311),
    (0x1316, 0x1317),
    (0x135B, 0x135C),
    (0x137D, 0x137F),
    (0x139A, 0x139F),
    (0x13F6, 0x13F7),
    (0x13FE, 0x13FF),
    (0x169D, 0x169F),
    (0x16F9, 0x16FF),
    (0x1716, 0x171E),
    (0x1737, 0x173F),
    (0x1754, 0x175F),
    (0x176D, 0x176D),
    (0x1771, 0x1771),
    (0x1774, 0x177F),
    (0x17DE, 0x17DF),
    (0x17EA, 0x17EF),
    (0x17FA, 0x17FF),
    (0x181A, 0x181F),
    (0x1879, 0x187F),
    (0x18AB, 0x18AF),
    (0x18F6, 0x18FF),
    (0x191F, 0x191F),
    (0x192C, 0x192F),
    (0x193C, 0x193F),
    (0x1941, 0x1943),
    (0x196E, 0x196F),
    (0x1975, 0x197F),
    (0x19AC, 0x19AF),
    (0x19CA, 0x19CF),
    (0x19DB, 0x19DD),
    (0x1A1C, 0x1A1D),
    (0x1A5F, 0x1A5F),
    (0x1A7D, 0x1A7E),
    (0x1A8A, 0x1A8F),
    (0x1A9A, 0x1A9F),
    (0x1AAE, 0x1AAF),
    (0x1ADE, 0x1ADF),
    (0x1AEC, 0x1AFF),
    (0x1B4D, 0x1B4D),
    (0x1BF4, 0x1BFB),
    (0x1C38, 0x1C3A),
    (0x1C4A, 0x1C4C),
    (0x1C8B, 0x1C8F),
    (0x1CBB, 0x1CBC),
    (0x1CC8, 0x1CCF),
    (0x1CFB, 0x1CFF),
    (0x1F16, 0x1F17),
    (0x1F1E, 0x1F1F),
    (0x1F46, 0x1F47),
    (0x1F4E, 0x1F4F),
    (0x1F58, 0x1F58),
    (0x1F5A, 0x1F5A),
    (0x1F5C, 0x1F5C),
    (0x1F5E, 0x1F5E),
    (0x1F7E, 0x1F7F),
    (0x1FB5, 0x1FB5),
    (0x1FC5, 0x1FC5),
    (0x1FD4, 0x1FD5),
    (0x1FDC, 0x1FDC),
    (0x1FF0, 0x1FF1),
    (0x1FF5, 0x1FF5),
    (0x1FFF, 0x1FFF),
    (0x2072, 0x2073),
    (0x208F, 0x208F),
    (0x209D, 0x209F),
    (0x20C2, 0x20CF),
    (0x20F1, 0x20FF),
    (0x218C, 0x218F),
    (0x242A, 0x243F),
    (0x244B, 0x245F),
    (0x2B74, 0x2B75),
    (0x2CF4, 0x2CF8),
    (0x2D26, 0x2D26),
    (0x2D28, 0x2D2C),
    (0x2D2E, 0x2D2F),
    (0x2D68, 0x2D6E),
    (0x2D71, 0x2D7E),
    (0x2D97, 0x2D9F),
    (0x2DA7, 0x2DA7),
    (0x2DAF, 0x2DAF),
    (0x2DB7, 0x2DB7),
    (0x2DBF, 0x2DBF),
    (0x2DC7, 0x2DC7),
    (0x2DCF, 0x2DCF),
    (0x2DD7, 0x2DD7),
    (0x2DDF, 0x2DDF),
    (0x2E5E, 0x2E7F),
    (0x2E9A, 0x2E9A),
    (0x2EF4, 0x2EFF),
    (0x2FD6, 0x2FEF),
    (0x3040, 0x3040),
    (0x3097, 0x3098),
    (0x3100, 0x3104),
    (0x3130, 0x3130),
    (0x318F, 0x318F),
    (0x31E6, 0x31EE),
    (0x321F, 0x321F),
    (0xA48D, 0xA48F),
    (0xA4C7, 0xA4CF),
    (0xA62C, 0xA63F),
    (0xA6F8, 0xA6FF),
    (0xA7DD, 0xA7F0),
    (0xA82D, 0xA82F),
    (0xA83A, 0xA83F),
    (0xA878, 0xA87F),
    (0xA8C6, 0xA8CD),
    (0xA8DA, 0xA8DF),
    (0xA954, 0xA95E),
    (0xA97D, 0xA97F),
    (0xA9CE, 0xA9CE),
    (0xA9DA, 0xA9DD),
    (0xA9FF, 0xA9FF),
    (0xAA37, 0xAA3F),
    (0xAA4E, 0xAA4F),
    (0xAA5A, 0xAA5B),
    (0xAAC3, 0xAADA),
    (0xAAF7, 0xAB00),
    (0xAB07, 0xAB08),
    (0xAB0F, 0xAB10),
    (0xAB17, 0xAB1F),
    (0xAB27, 0xAB27),
    (0xAB2F, 0xAB2F),
    (0xAB6C, 0xAB6F),
    (0xABEE, 0xABEF),
    (0xABFA, 0xABFF),
    (0xD7A4, 0xD7AF),
    (0xD7C7, 0xD7CA),
    (0xD7FC, 0xD7FF),
    (0xFA6E, 0xFA6F),
    (0xFADA, 0xFAFF),
    (0xFB07, 0xFB12),
    (0xFB18, 0xFB1C),
    (0xFB37, 0xFB37),
    (0xFB3D, 0xFB3D),
    (0xFB3F, 0xFB3F),
    (0xFB42, 0xFB42),
    (0xFB45, 0xFB45),
    (0xFDD0, 0xFDEF),
    (0xFE1A, 0xFE1F),
    (0xFE53, 0xFE53),
    (0xFE67, 0xFE67),
    (0xFE6C, 0xFE6F),
    (0xFE75, 0xFE75),
    (0xFEFD, 0xFEFE),
    (0xFF00, 0xFF00),
    (0xFFBF, 0xFFC1),
    (0xFFC8, 0xFFC9),
    (0xFFD0, 0xFFD1),
    (0xFFD8, 0xFFD9),
    (0xFFDD, 0xFFDF),
    (0xFFE7, 0xFFE7),
    (0xFFEF, 0xFFEF),
    (0xFFFE, 0xFFFF),
    (0x1000C, 0x1000C),
    (0x10027, 0x10027),
    (0x1003B, 0x1003B),
    (0x1003E, 0x1003E),
    (0x1004E, 0x1004F),
    (0x1005E, 0x1007F),
    (0x100FB, 0x100FF),
    (0x10103, 0x10106),
    (0x10134, 0x10136),
    (0x1018F, 0x1018F),
    (0x1019D, 0x1019F),
    (0x101A1, 0x101CF),
    (0x101FE, 0x1027F),
    (0x1029D, 0x1029F),
    (0x102D1, 0x102DF),
    (0x102FC, 0x102FF),
    (0x10324, 0x1032C),
    (0x1034B, 0x1034F),
    (0x1037B, 0x1037F),
    (0x1039E, 0x1039E),
    (0x103C4, 0x103C7),
    (0x103D6, 0x103FF),
    (0x1049E, 0x1049F),
    (0x104AA, 0x104AF),
    (0x104D4, 0x104D7),
    (0x104FC, 0x104FF),
    (0x10528, 0x1052F),
    (0x10564, 0x1056E),
    (0x1057B, 0x1057B),
    (0x1058B, 0x1058B),
    (0x10593, 0x10593),
    (0x10596, 0x10596),
    (0x105A2, 0x105A2),
    (0x105B2, 0x105B2),
    (0x105BA, 0x105BA),
    (0x105BD, 0x105BF),
    (0x105F4, 0x105FF),
    (0x10737, 0x1073F),
    (0x10756, 0x1075F),
    (0x10768, 0x1077F),
    (0x10786, 0x10786),
    (0x107B1, 0x107B1),
    (0x107BB, 0x107FF),
    (0x10806, 0x10807),
    (0x10809, 0x10809),
    (0x10836, 0x10836),
    (0x10839, 0x1083B),
    (0x1083D, 0x1083E),
    (0x10856, 0x10856),
    (0x1089F, 0x108A6),
    (0x108B0, 0x108DF),
    (0x108F3, 0x108F3),
    (0x108F6, 0x108FA),
    (0x1091C, 0x1091E),
    (0x1093A, 0x1093E),
    (0x1095A, 0x1097F),
    (0x109B8, 0x109BB),
    (0x109D0, 0x109D1),
    (0x10A04, 0x10A04),
    (0x10A07, 0x10A0B),
    (0x10A14, 0x10A14),
    (0x10A18, 0x10A18),
    (0x10A36, 0x10A37),
    (0x10A3B, 0x10A3E),
    (0x10A49, 0x10A4F),
    (0x10A59, 0x10A5F),
    (0x10AA0, 0x10ABF),
    (0x10AE7, 0x10AEA),
    (0x10AF7, 0x10AFF),
    (0x10B36, 0x10B38),
    (0x10B56, 0x10B57),
    (0x10B73, 0x10B77),
    (0x10B92, 0x10B98),
    (0x10B9D, 0x10BA8),
    (0x10BB0, 0x10BFF),
    (0x10C49, 0x10C7F),
    (0x10CB3, 0x10CBF),
    (0x10CF3, 0x10CF9),
    (0x10D28, 0x10D2F),
    (0x10D3A, 0x10D3F),
    (0x10D66, 0x10D68),
    (0x10D86, 0x10D8D),
    (0x10D90, 0x10E5F),
    (0x10E7F, 0x10E7F),
    (0x10EAA, 0x10EAA),
    (0x10EAE, 0x10EAF),
    (0x10EB2, 0x10EC1),
    (0x10EC8, 0x10ECF),
    (0x10ED9, 0x10EF9),
    (0x10F28, 0x10F2F),
    (0x10F5A, 0x10F6F),
    (0x10F8A, 0x10FAF),
    (0x10FCC, 0x10FDF),
    (0x10FF7, 0x10FFF),
    (0x1104E, 0x11051),
    (0x11076, 0x1107E),
    (0x110C3, 0x110CC),
    (0x110CE, 0x110CF),
    (0x110E9, 0x110EF),
    (0x110FA, 0x110FF),
    (0x11135, 0x11135),
    (0x11148, 0x1114F),
    (0x11177, 0x1117F),
    (0x111E0, 0x111E0),
    (0x111F5, 0x111FF),
    (0x11212, 0x11212),
    (0x11242, 0x1127F),
    (0x11287, 0x11287),
    (0x11289, 0x11289),
    (0x1128E, 0x1128E),
    (0x1129E, 0x1129E),
    (0x112AA, 0x112AF),
    (0x112EB, 0x112EF),
    (0x112FA, 0x112FF),
    (0x11304, 0x11304),
    (0x1130D, 0x1130E),
    (0x11311, 0x11312),
    (0x11329, 0x11329),
    (0x11331, 0x11331),
    (0x11334, 0x11334),
    (0x1133A, 0x1133A),
    (0x11345, 0x11346),
    (0x11349, 0x1134A),
    (0x1134E, 0x1134F),
    (0x11351, 0x11356),
    (0x11358, 0x1135C),
    (0x11364, 0x11365),
    (0x1136D, 0x1136F),
    (0x11375, 0x1137F),
    (0x1138A, 0x1138A),
    (0x1138C, 0x1138D),
    (0x1138F, 0x1138F),
    (0x113B6, 0x113B6),
    (0x113C1, 0x113C1),
    (0x113C3, 0x113C4),
    (0x113C6, 0x113C6),
    (0x113CB, 0x113CB),
    (0x113D6, 0x113D6),
    (0x113D9, 0x113E0),
    (0x113E3, 0x113FF),
    (0x1145C, 0x1145C),
    (0x11462, 0x1147F),
    (0x114C8, 0x114CF),
    (0x114DA, 0x1157F),
    (0x115B6, 0x115B7),
    (0x115DE, 0x115FF),
    (0x11645, 0x1164F),
    (0x1165A, 0x1165F),
    (0x1166D, 0x1167F),
    (0x116BA, 0x116BF),
    (0x116CA, 0x116CF),
    (0x116E4, 0x116FF),
    (0x1171B, 0x1171C),
    (0x1172C, 0x1172F),
    (0x11747, 0x117FF),
    (0x1183C, 0x1189F),
    (0x118F3, 0x118FE),
    (0x11907, 0x11908),
    (0x1190A, 0x1190B),
    (0x11914, 0x11914),
    (0x11917, 0x11917),
    (0x11936, 0x11936),
    (0x11939, 0x1193A),
    (0x11947, 0x1194F),
    (0x1195A, 0x1199F),
    (0x119A8, 0x119A9),
    (0x119D8, 0x119D9),
    (0x119E5, 0x119FF),
    (0x11A48, 0x11A4F),
    (0x11AA3, 0x11AAF),
    (0x11AF9, 0x11AFF),
    (0x11B0A, 0x11B5F),
    (0x11B68, 0x11BBF),
    (0x11BE2, 0x11BEF),
    (0x11BFA, 0x11BFF),
    (0x11C09, 0x11C09),
    (0x11C37, 0x11C37),
    (0x11C46, 0x11C4F),
    (0x11C6D, 0x11C6F),
    (0x11C90, 0x11C91),
    (0x11CA8, 0x11CA8),
    (0x11CB7, 0x11CFF),
    (0x11D07, 0x11D07),
    (0x11D0A, 0x11D0A),
    (0x11D37, 0x11D39),
    (0x11D3B, 0x11D3B),
    (0x11D3E, 0x11D3E),
    (0x11D48, 0x11D4F),
    (0x11D5A, 0x11D5F),
    (0x11D66, 0x11D66),
    (0x11D69, 0x11D69),
    (0x11D8F, 0x11D8F),
    (0x11D92, 0x11D92),
    (0x11D99, 0x11D9F),
    (0x11DAA, 0x11DAF),
    (0x11DDC, 0x11DDF),
    (0x11DEA, 0x11EDF),
    (0x11EF9, 0x11EFF),
    (0x11F11, 0x11F11),
    (0x11F3B, 0x11F3D),
    (0x11F5B, 0x11FAF),
    (0x11FB1, 0x11FBF),
    (0x11FF2, 0x11FFE),
    (0x1239A, 0x123FF),
    (0x1246F, 0x1246F),
    (0x12475, 0x1247F),
    (0x12544, 0x12F8F),
    (0x12FF3, 0x12FFF),
    (0x13456, 0x1345F),
    (0x143FB, 0x143FF),
    (0x14647, 0x160FF),
    (0x1613A, 0x167FF),
    (0x16A39, 0x16A3F),
    (0x16A5F, 0x16A5F),
    (0x16A6A, 0x16A6D),
    (0x16ABF, 0x16ABF),
    (0x16ACA, 0x16ACF),
    (0x16AEE, 0x16AEF),
    (0x16AF6, 0x16AFF),
    (0x16B46, 0x16B4F),
    (0x16B5A, 0x16B5A),
    (0x16B62, 0x16B62),
    (0x16B78, 0x16B7C),
    (0x16B90, 0x16D3F),
    (0x16D7A, 0x16E3F),
    (0x16E9B, 0x16E9F),
    (0x16EB9, 0x16EBA),
    (0x16ED4, 0x16EFF),
    (0x16F4B, 0x16F4E),
    (0x16F88, 0x16F8E),
    (0x16FA0, 0x16FDF),
    (0x16FE5, 0x16FEF),
    (0x16FF7, 0x16FFF),
    (0x18CD6, 0x18CFE),
    (0x18D1F, 0x18D7F),
    (0x18DF3, 0x1AFEF),
    (0x1AFF4, 0x1AFF4),
    (0x1AFFC, 0x1AFFC),
    (0x1AFFF, 0x1AFFF),
    (0x1B123, 0x1B131),
    (0x1B133, 0x1B14F),
    (0x1B153, 0x1B154),
    (0x1B156, 0x1B163),
    (0x1B168, 0x1B16F),
    (0x1B2FC, 0x1BBFF),
    (0x1BC6B, 0x1BC6F),
    (0x1BC7D, 0x1BC7F),
    (0x1BC89, 0x1BC8F),
    (0x1BC9A, 0x1BC9B),
    (0x1BCA4, 0x1CBFF),
    (0x1CCFD, 0x1CCFF),
    (0x1CEB4, 0x1CEB9),
    (0x1CED1, 0x1CEDF),
    (0x1CEF1, 0x1CEFF),
    (0x1CF2E, 0x1CF2F),
    (0x1CF47, 0x1CF4F),
    (0x1CFC4, 0x1CFFF),
    (0x1D0F6, 0x1D0FF),
    (0x1D127, 0x1D128),
    (0x1D1EB, 0x1D1FF),
    (0x1D246, 0x1D2BF),
    (0x1D2D4, 0x1D2DF),
    (0x1D2F4, 0x1D2FF),
    (0x1D357, 0x1D35F),
    (0x1D379, 0x1D3FF),
    (0x1D455, 0x1D455),
    (0x1D49D, 0x1D49D),
    (0x1D4A0, 0x1D4A1),
    (0x1D4A3, 0x1D4A4),
    (0x1D4A7, 0x1D4A8),
    (0x1D4AD, 0x1D4AD),
    (0x1D4BA, 0x1D4BA),
    (0x1D4BC, 0x1D4BC),
    (0x1D4C4, 0x1D4C4),
    (0x1D506, 0x1D506),
    (0x1D50B, 0x1D50C),
    (0x1D515, 0x1D515),
    (0x1D51D, 0x1D51D),
    (0x1D53A, 0x1D53A),
    (0x1D53F, 0x1D53F),
    (0x1D545, 0x1D545),
    (0x1D547, 0x1D549),
    (0x1D551, 0x1D551),
    (0x1D6A6, 0x1D6A7),
    (0x1D7CC, 0x1D7CD),
    (0x1DA8C, 0x1DA9A),
    (0x1DAA0, 0x1DAA0),
    (0x1DAB0, 0x1DEFF),
    (0x1DF1F, 0x1DF24),
    (0x1DF2B, 0x1DFFF),
    (0x1E007, 0x1E007),
    (0x1E019, 0x1E01A),
    (0x1E022, 0x1E022),
    (0x1E025, 0x1E025),
    (0x1E02B, 0x1E02F),
    (0x1E06E, 0x1E08E),
    (0x1E090, 0x1E0FF),
    (0x1E12D, 0x1E12F),
    (0x1E13E, 0x1E13F),
    (0x1E14A, 0x1E14D),
    (0x1E150, 0x1E28F),
    (0x1E2AF, 0x1E2BF),
    (0x1E2FA, 0x1E2FE),
    (0x1E300, 0x1E4CF),
    (0x1E4FA, 0x1E5CF),
    (0x1E5FB, 0x1E5FE),
    (0x1E600, 0x1E6BF),
    (0x1E6DF, 0x1E6DF),
    (0x1E6F6, 0x1E6FD),
    (0x1E700, 0x1E7DF),
    (0x1E7E7, 0x1E7E7),
    (0x1E7EC, 0x1E7EC),
    (0x1E7EF, 0x1E7EF),
    (0x1E7FF, 0x1E7FF),
    (0x1E8C5, 0x1E8C6),
    (0x1E8D7, 0x1E8FF),
    (0x1E94C, 0x1E94F),
    (0x1E95A, 0x1E95D),
    (0x1E960, 0x1EC70),
    (0x1ECB5, 0x1ED00),
    (0x1ED3E, 0x1EDFF),
    (0x1EE04, 0x1EE04),
    (0x1EE20, 0x1EE20),
    (0x1EE23, 0x1EE23),
    (0x1EE25, 0x1EE26),
    (0x1EE28, 0x1EE28),
    (0x1EE33, 0x1EE33),
    (0x1EE38, 0x1EE38),
    (0x1EE3A, 0x1EE3A),
    (0x1EE3C, 0x1EE41),
    (0x1EE43, 0x1EE46),
    (0x1EE48, 0x1EE48),
    (0x1EE4A, 0x1EE4A),
    (0x1EE4C, 0x1EE4C),
    (0x1EE50, 0x1EE50),
    (0x1EE53, 0x1EE53),
    (0x1EE55, 0x1EE56),
    (0x1EE58, 0x1EE58),
    (0x1EE5A, 0x1EE5A),
    (0x1EE5C, 0x1EE5C),
    (0x1EE5E, 0x1EE5E),
    (0x1EE60, 0x1EE60),
    (0x1EE63, 0x1EE63),
    (0x1EE65, 0x1EE66),
    (0x1EE6B, 0x1EE6B),
    (0x1EE73, 0x1EE73),
    (0x1EE78, 0x1EE78),
    (0x1EE7D, 0x1EE7D),
    (0x1EE7F, 0x1EE7F),
    (0x1EE8A, 0x1EE8A),
    (0x1EE9C, 0x1EEA0),
    (0x1EEA4, 0x1EEA4),
    (0x1EEAA, 0x1EEAA),
    (0x1EEBC, 0x1EEEF),
    (0x1EEF2, 0x1EFFF),
    (0x1F02C, 0x1F02F),
    (0x1F094, 0x1F09F),
    (0x1F0AF, 0x1F0B0),
    (0x1F0C0, 0x1F0C0),
    (0x1F0D0, 0x1F0D0),
    (0x1F0F6, 0x1F0FF),
    (0x1F1AE, 0x1F1E5),
    (0x1F203, 0x1F20F),
    (0x1F23C, 0x1F23F),
    (0x1F249, 0x1F24F),
    (0x1F252, 0x1F25F),
    (0x1F266, 0x1F2FF),
    (0x1F6D9, 0x1F6DB),
    (0x1F6ED, 0x1F6EF),
    (0x1F6FD, 0x1F6FF),
    (0x1F7DA, 0x1F7DF),
    (0x1F7EC, 0x1F7EF),
    (0x1F7F1, 0x1F7FF),
    (0x1F80C, 0x1F80F),
    (0x1F848, 0x1F84F),
    (0x1F85A, 0x1F85F),
    (0x1F888, 0x1F88F),
    (0x1F8AE, 0x1F8AF),
    (0x1F8BC, 0x1F8BF),
    (0x1F8C2, 0x1F8CF),
    (0x1F8D9, 0x1F8FF),
    (0x1FA58, 0x1FA5F),
    (0x1FA6E, 0x1FA6F),
    (0x1FA7D, 0x1FA7F),
    (0x1FA8B, 0x1FA8D),
    (0x1FAC7, 0x1FAC7),
    (0x1FAC9, 0x1FACC),
    (0x1FADD, 0x1FADE),
    (0x1FAEB, 0x1FAEE),
    (0x1FAF9, 0x1FAFF),
    (0x1FB93, 0x1FB93),
    (0x1FBFB, 0x1FFFF),
    (0x2A6E0, 0x2A6FF),
    (0x2B81E, 0x2B81F),
    (0x2CEAE, 0x2CEAF),
    (0x2EBE1, 0x2EBEF),
    (0x2EE5E, 0x2F7FF),
    (0x2FA1E, 0x2FFFF),
    (0x3134B, 0x3134F),
    (0x3347A, 0xDFFFF),
    (0xE1000, 0xEFFFF),
    (0xFFFFE, 0xFFFFF),
    (0x10FFFE, 0x10FFFF),
];
// END GENERATED TABLES

/// Whether `v` lies in one of `table`'s ranges (binary search; the table is sorted).
fn in_table(table: &[ScalarRange], v: u32) -> bool {
    table
        .binary_search_by(|&(first, last)| {
            if last < v {
                Ordering::Less
            } else if first > v {
                Ordering::Greater
            } else {
                Ordering::Equal
            }
        })
        .is_ok()
}

/// The class that makes `c` escaped, or `None` when `c` is shown as written.
/// Pure table lookup; never calls into std's Unicode tables.
#[must_use]
pub fn escape_class(c: char) -> Option<EscapeClass> {
    let v = u32::from(c);
    if in_table(CONTROL, v) {
        Some(EscapeClass::Control)
    } else if in_table(INVISIBLE, v) {
        Some(EscapeClass::Invisible)
    } else if in_table(PRIVATE_USE, v) {
        Some(EscapeClass::PrivateUse)
    } else if in_table(UNASSIGNED, v) {
        Some(EscapeClass::Unassigned)
    } else {
        None
    }
}

/// Whether [`escape_controls`] (and [`escape_quoted`]) write `c` as an escape.
#[must_use]
pub fn is_escaped(c: char) -> bool {
    escape_class(c).is_some()
}

/// Append the escape for `c` to `out`, whether or not `c` is in the escaped set: `\0`, `\t`,
/// `\n`, `\r` for U+0000, U+0009, U+000A, U+000D, and `\u{` + the scalar value in minimal
/// lower-case hexadecimal + `}` for every other char (`\u{1b}`, `\u{202e}`, `\u{e01ef}`).
/// This rendering is this crate's contract, not std's. It equals rustc 1.99.1's
/// `char::escape_debug` for every escaped char, and 1.96.0's for every char 1.96.0 escaped
/// (measured over every code point, S125b), so adopting it changed no byte std already escaped.
pub fn push_escape(out: &mut String, c: char) {
    match c {
        '\0' => out.push_str("\\0"),
        '\t' => out.push_str("\\t"),
        '\n' => out.push_str("\\n"),
        '\r' => out.push_str("\\r"),
        _ => {
            // Writing to a String cannot fail.
            let _ = write!(out, "\\u{{{:x}}}", u32::from(c));
        }
    }
}

/// MB-8: `text` with every escaped char (see [`escape_class`]) written by [`push_escape`];
/// everything else, including quotes, backslashes, backticks, letters, combining marks and
/// emoji, is copied as written. Position-independent: a combining mark is never escaped, at the
/// start of the text or anywhere else. Used on text a mappings file can put on a terminal:
/// serde's error message and a mode name (`play`'s "Engine mode" line, the TUI's mode).
#[must_use]
pub fn escape_controls(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if is_escaped(c) {
            push_escape(&mut out, c);
        } else {
            out.push(c);
        }
    }
    out
}

/// [`escape_controls`] plus `"` → `\"`, `'` → `\'` and `\` → `\\`: for text shown inside
/// back-quotes in a sentence (a file name, a JSON key or path, a form id), where a quote or a
/// backslash in the text must stay distinguishable from the sentence's own punctuation. For
/// every char except a grapheme extender at the very start of the text, this is what
/// `str::escape_debug` printed on rustc 1.99.1; a LEADING grapheme extender (a key that begins
/// with U+0301) is copied as written instead of escaped.
#[must_use]
pub fn escape_quoted(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\'' => out.push_str("\\'"),
            '\\' => out.push_str("\\\\"),
            _ if is_escaped(c) => push_escape(&mut out, c),
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const TABLES: [(&str, &[ScalarRange], EscapeClass); 4] = [
        ("CONTROL", CONTROL, EscapeClass::Control),
        ("INVISIBLE", INVISIBLE, EscapeClass::Invisible),
        ("PRIVATE_USE", PRIVATE_USE, EscapeClass::PrivateUse),
        ("UNASSIGNED", UNASSIGNED, EscapeClass::Unassigned),
    ];

    /// Every scalar value, in ascending order.
    fn all_chars() -> impl Iterator<Item = char> {
        (0..=0x10_FFFFu32).filter_map(char::from_u32)
    }

    /// What `str::escape_debug` on the BUILDING toolchain does with `c` when it is not first in
    /// its string (first position adds a grapheme-extend rule this crate does not have). This is
    /// the probe the escaper used before S125b; it now lives only here, in the tests.
    fn std_escapes(c: char) -> bool {
        let mut probe = String::with_capacity(5);
        probe.push(' ');
        probe.push(c);
        !probe.escape_debug().eq([' ', c])
    }

    /// The tables are sorted, non-overlapping, non-adjacent, surrogate-free, and the version is
    /// the one the generator pins.
    #[test]
    fn tables_are_well_formed() {
        assert_eq!(TABLES_UNICODE_VERSION, (17, 0, 0));
        for (name, table, _) in TABLES {
            assert!(!table.is_empty(), "{name} is empty");
            for (i, &(first, last)) in table.iter().enumerate() {
                assert!(first <= last, "{name}[{i}] is reversed");
                assert!(last <= 0x10_FFFF, "{name}[{i}] is past U+10FFFF");
                assert!(
                    last < 0xD800 || first > 0xDFFF,
                    "{name}[{i}] contains a surrogate"
                );
                if i > 0 {
                    assert!(
                        table[i - 1].1 + 1 < first,
                        "{name}[{i}] overlaps or touches the row before it (tables are merged)"
                    );
                }
            }
        }
    }

    /// T3 (design s125b-01) / SH9 "table alone": the escaped set is a reviewed constant, not a
    /// property of rustc. Every scalar value is classified from the tables alone (std is not
    /// consulted), each is in at most one table, and the per-class counts and a digest of the
    /// whole set match the generator's run on UCD 17.0.0. Deleting, widening or narrowing any
    /// row changes a count or the digest, on every toolchain.
    #[test]
    fn escape_set_is_frozen() {
        let mut counts = [0usize; 4];
        let mut digest: u64 = 0xCBF2_9CE4_8422_2325; // FNV-1a-64 offset basis
        for c in all_chars() {
            let v = u32::from(c);
            let member: Vec<usize> = (0..4).filter(|&k| in_table(TABLES[k].1, v)).collect();
            assert!(member.len() <= 1, "U+{v:04X} is in more than one table");
            if let Some(&k) = member.first() {
                assert_eq!(escape_class(c), Some(TABLES[k].2), "U+{v:04X}");
                counts[k] += 1;
                for byte in v.to_le_bytes() {
                    digest ^= u64::from(byte);
                    digest = digest.wrapping_mul(0x0000_0100_0000_01B3);
                }
            } else {
                assert_eq!(escape_class(c), None, "U+{v:04X}");
            }
        }
        assert_eq!(
            counts, EXPECTED_COUNTS,
            "per-class counts [Control, Invisible, PrivateUse, Unassigned]"
        );
        assert_eq!(counts.iter().sum::<usize>(), EXPECTED_TOTAL);
        assert_eq!(digest, EXPECTED_DIGEST, "digest 0x{digest:016x}");
    }

    /// The constants `escape_set_is_frozen` asserts, as printed by the generator.
    const EXPECTED_COUNTS: [usize; 4] = [65, 4_224, 137_468, 810_961];
    const EXPECTED_TOTAL: usize = 952_718;
    const EXPECTED_DIGEST: u64 = 0x3fc6_ecf7_a709_c55f;

    /// SH9 T-B: every security class is in the table, by named representatives AND range
    /// endpoints, checked against the tables alone (no std). This is the test that reads a hole
    /// in the table on a toolchain whose std happens to escape the same character.
    #[test]
    fn every_security_class_is_in_the_table() {
        let classes: &[(&str, EscapeClass, &[u32])] = &[
            (
                "C0, DEL, C1 (ESC, BEL, NEL, the 8-bit CSI)",
                EscapeClass::Control,
                &[0x00, 0x07, 0x1B, 0x1F, 0x7F, 0x85, 0x9B, 0x9F],
            ),
            (
                "bidi overrides, isolates and marks",
                EscapeClass::Invisible,
                &[
                    0x061C, 0x200E, 0x200F, 0x202A, 0x202B, 0x202C, 0x202D, 0x202E, 0x2066, 0x2067,
                    0x2068, 0x2069,
                ],
            ),
            (
                "line and paragraph separators",
                EscapeClass::Invisible,
                &[0x2028, 0x2029],
            ),
            (
                "zero-width, joiners, word joiner, invisible operators, deprecated format",
                EscapeClass::Invisible,
                &[
                    0x200B, 0x200C, 0x200D, 0x2060, 0x2061, 0x2064, 0x2065, 0x206A, 0x206F,
                ],
            ),
            ("BOM / ZWNBSP", EscapeClass::Invisible, &[0xFEFF]),
            (
                "soft hyphen, combining grapheme joiner, Mongolian vowel separator",
                EscapeClass::Invisible,
                &[0x00AD, 0x034F, 0x180E],
            ),
            (
                "variation selectors (VS1-16, VS17-256, Mongolian FVS)",
                EscapeClass::Invisible,
                &[
                    0xFE00, 0xFE0E, 0xFE0F, 0xE0100, 0xE0101, 0xE01EF, 0x180B, 0x180C, 0x180D,
                    0x180F,
                ],
            ),
            (
                "tag characters",
                EscapeClass::Invisible,
                &[0xE0000, 0xE0001, 0xE0020, 0xE0041, 0xE0067, 0xE007F],
            ),
            (
                "Hangul fillers and Khmer inherent vowels (blank letters)",
                EscapeClass::Invisible,
                &[0x115F, 0x1160, 0x3164, 0xFFA0, 0x17B4, 0x17B5],
            ),
            (
                "other default-ignorables",
                EscapeClass::Invisible,
                &[0xFFF0, 0xFFF8, 0x1BCA0, 0x1BCA3, 0x1D173, 0x1D17A, 0xE0FFF],
            ),
            (
                "other format characters (incl. interlinear annotation)",
                EscapeClass::Invisible,
                &[
                    0x0600, 0x0605, 0x06DD, 0x070F, 0x0890, 0x0891, 0x08E2, 0xFFF9, 0xFFFB,
                    0x110BD, 0x110CD, 0x13430, 0x1343F,
                ],
            ),
            (
                "non-ASCII spaces",
                EscapeClass::Invisible,
                &[0x00A0, 0x1680, 0x2000, 0x200A, 0x202F, 0x205F, 0x3000],
            ),
            (
                "private use",
                EscapeClass::PrivateUse,
                &[0xE000, 0xF8FF, 0xF0000, 0xFFFFD, 0x100000, 0x10FFFD],
            ),
            (
                "unassigned in Unicode 17.0.0 (incl. noncharacters)",
                EscapeClass::Unassigned,
                &[0x0378, 0xFDD0, 0xFDEF, 0xFFFE, 0xFFFF, 0x10FFFF, 0xEFFFD],
            ),
        ];
        let mut holes = Vec::new();
        for (name, class, reps) in classes {
            for &v in *reps {
                let c = char::from_u32(v).expect("a scalar value");
                if escape_class(c) != Some(*class) {
                    holes.push(format!(
                        "U+{v:04X} ({name}): expected {class:?}, table says {:?}",
                        escape_class(c)
                    ));
                }
            }
        }
        assert!(
            holes.is_empty(),
            "holes in the frozen set:\n{}",
            holes.join("\n")
        );
        // And what must stay as written.
        for c in [
            ' ', 'a', 'Z', '0', '`', '"', '\'', '\\', 'é', '\u{301}', '\u{94d}', '✔', '雅',
        ] {
            assert_eq!(
                escape_class(c),
                None,
                "U+{:04X} must be shown",
                u32::from(c)
            );
        }
    }

    /// T5: the rendering is pinned by literals, not by std.
    #[test]
    fn push_escape_renders_by_its_own_rule() {
        let cases: &[(char, &str)] = &[
            ('\0', "\\0"),
            ('\t', "\\t"),
            ('\n', "\\n"),
            ('\r', "\\r"),
            ('\u{1}', "\\u{1}"),
            ('\u{1b}', "\\u{1b}"),
            ('\u{7f}', "\\u{7f}"),
            ('\u{9b}', "\\u{9b}"),
            ('\u{a0}', "\\u{a0}"),
            ('\u{202e}', "\\u{202e}"),
            ('\u{fe0f}', "\\u{fe0f}"),
            ('\u{e01ef}', "\\u{e01ef}"),
            ('\u{10fffd}', "\\u{10fffd}"),
        ];
        for &(c, want) in cases {
            let mut out = String::new();
            push_escape(&mut out, c);
            assert_eq!(out, want, "U+{:04X}", u32::from(c));
        }
    }

    /// `escape_quoted` adds exactly the three quote/backslash escapes to `escape_controls`, and a
    /// combining mark is kept wherever it stands.
    #[test]
    fn escape_quoted_escapes_quotes_and_backslashes_too() {
        assert_eq!(
            escape_quoted("C:\\a \"b\" 'c' \u{1b}[31m\u{202e}\u{fe0f}"),
            "C:\\\\a \\\"b\\\" \\'c\\' \\u{1b}[31m\\u{202e}\\u{fe0f}"
        );
        assert_eq!(escape_controls("\"q\" 'q' a\\b"), "\"q\" 'q' a\\b");
        assert_eq!(escape_quoted("\u{301}e"), "\u{301}e");
        assert_eq!(escape_controls("\u{301}e"), "\u{301}e");
        assert_eq!(escape_quoted("plain-name_1.json"), "plain-name_1.json");
    }

    /// §5 S-1 (L-99): an escaped char FIRST in the text is escaped too, through both escapers.
    /// (std has a first-position special case, so position 0 is where a "keep std's semantics"
    /// edit would most plausibly reintroduce a hole.)
    #[test]
    fn an_escaped_char_at_the_start_of_the_text_is_escaped() {
        let cases: &[(&str, &str)] = &[
            ("\u{1b}[2J", "\\u{1b}[2J"),
            ("\u{202e}x", "\\u{202e}x"),
            ("\u{fe0f}", "\\u{fe0f}"),
            ("\u{fe0f}x", "\\u{fe0f}x"),
        ];
        for &(input, want) in cases {
            assert_eq!(escape_controls(input), want, "escape_controls({input:?})");
            assert_eq!(escape_quoted(input), want, "escape_quoted({input:?})");
        }
    }

    /// 06EA X4 (L-103): a backtick, the delimiter every `escape_quoted` site wraps a name in, does
    /// not shield the character after it: ESC, a bidi override and a variation selector right
    /// after a backtick are escaped, through both escapers.
    #[test]
    fn a_backtick_does_not_shield_the_next_char() {
        for c in ['\u{1b}', '\u{202e}', '\u{fe0f}'] {
            let want = format!("k`\\u{{{:x}}}[31m`", u32::from(c));
            let input = format!("k`{c}[31m`");
            assert_eq!(escape_quoted(&input), want, "escape_quoted({input:?})");
            assert_eq!(escape_controls(&input), want, "escape_controls({input:?})");
        }
    }

    /// §5 S-2 (L-99): CRATE-WIDE, no non-test code under `src/` calls std's char/str escapers
    /// (`escape_debug`, `escape_default`, `escape_unicode`), except `presets.rs`, which escapes
    /// only our own compiled-in manifest (L-94). On rustc 1.99.1 std escapes exactly the frozen
    /// set, so a std-routed escaper anywhere in the crate (a helper in another module, called
    /// from here or from `mapping_loader`) is invisible to every behavioural test there; this
    /// text scan is what reads it. "Test code" = an item under a column-0 `#[cfg(test)]` whose
    /// end [`test_item_end`] can place with certainty; anything it cannot place PANICS (fails RED),
    /// so a std call can never hide after a test item the scan misread (06EA C-1, X5).
    /// Comment lines are skipped. `{:?}` is NOT forbidden crate-wide (L-99).
    /// The index of the LAST line of the item a column-0 `#[cfg(test)]` at `at` applies to, or why
    /// it cannot be placed with certainty (06EA C-1: the crate-wide scan must FAIL CLOSED, never
    /// skip production code). Accepted shapes, rustfmt layout only:
    /// - a one-line item: ends in `;`, or ends in `}` with balanced braces (`fn marker() {}`);
    /// - a block item: the head (or a multi-line signature whose continuation lines are indented
    ///   or start with `)`) ends in `{`; every following line is empty or indented, up to a line
    ///   that is exactly `}`.
    ///
    /// Anything else (a column-0 `};` or `];`, a `where` clause, a non-indented line in the body,
    /// no item at all) is an error, and the scan panics.
    fn test_item_end(lines: &[&str], at: usize) -> Result<usize, String> {
        let indented = |l: &str| l.starts_with(' ') || l.starts_with('\t');
        let mut j = at + 1;
        while j < lines.len() && lines[j].starts_with("#[") {
            j += 1;
        }
        let head = *lines.get(j).ok_or("no item after it")?;
        if head.is_empty() || indented(head) || head.starts_with("//") {
            return Err(format!("`{head}` is not a column-0 item head"));
        }
        let h = head.trim_end();
        let (opens, closes) = (h.matches('{').count(), h.matches('}').count());
        if opens == closes && (h.ends_with(';') || (opens > 0 && h.ends_with('}'))) {
            return Ok(j);
        }
        let mut k = j;
        while !lines[k].trim_end().ends_with('{') {
            k += 1;
            if k >= lines.len() || k > j + 40 {
                return Err("no block opens within 40 lines of the head".into());
            }
            let l = lines[k];
            if !(indented(l) || l.starts_with(')')) || l.trim_end().ends_with(';') {
                return Err(format!(
                    "line {}: `{l}` is not a signature continuation",
                    k + 1
                ));
            }
        }
        for (m, &l) in lines.iter().enumerate().skip(k + 1) {
            if l.is_empty() || indented(l) {
                continue;
            }
            if l == "}" {
                return Ok(m);
            }
            return Err(format!(
                "line {}: `{l}` is not the item's column-0 `}}`",
                m + 1
            ));
        }
        Err("no column-0 `}` closes the item".into())
    }

    #[test]
    fn no_module_outside_presets_uses_std_escapers() {
        fn rs_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
            for e in std::fs::read_dir(dir).expect("read src") {
                let p = e.expect("dir entry").path();
                if p.is_dir() {
                    rs_files(&p, out);
                } else if p.extension().is_some_and(|x| x == "rs") {
                    out.push(p);
                }
            }
        }
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut files = Vec::new();
        rs_files(&root, &mut files);
        files.sort();
        assert!(
            files.len() > 20,
            "found only {} files under src/",
            files.len()
        );
        let allow = root.join("presets.rs");
        let tokens = ["escape_debug", "escape_default", "escape_unicode"];
        let mut hits = Vec::new();
        let mut scanned = 0;
        for f in &files {
            if *f == allow {
                continue;
            }
            scanned += 1;
            let text = std::fs::read_to_string(f).expect("read file");
            let lines: Vec<&str> = text.lines().collect();
            let mut i = 0;
            while i < lines.len() {
                if lines[i] == "#[cfg(test)]" {
                    let end = test_item_end(&lines, i).unwrap_or_else(|why| {
                        panic!(
                            "{}: cannot place the end of the #[cfg(test)] item at line {} \
                             ({why}); the scan fails closed",
                            f.display(),
                            i + 1
                        )
                    });
                    i = end + 1;
                    continue;
                }
                let line = lines[i];
                if !line.trim_start().starts_with("//") {
                    for t in tokens {
                        if line.contains(t) {
                            hits.push(format!(
                                "{}:{}: `{t}` in `{}`",
                                f.display(),
                                i + 1,
                                line.trim()
                            ));
                        }
                    }
                }
                i += 1;
            }
        }
        assert!(scanned > 20, "scanned only {scanned} files");
        assert!(
            hits.is_empty(),
            "std escapers outside presets.rs:\n{}",
            hits.join("\n")
        );
    }

    /// T4 (the drift detector): if the BUILDING toolchain's `str::escape_debug` escapes a char,
    /// the frozen set escapes it too. std escaping something we print as written is a possible
    /// hole and FAILS; the converse (a newly assigned char we still escape) is safe and only
    /// reported (next test). Runs on whatever toolchain builds the tests: pinned in CI, floating
    /// stable in the weekly drift job.
    #[test]
    fn std_never_escapes_what_we_show() {
        let shown_by_us: Vec<u32> = all_chars()
            .filter(|&c| !matches!(c, '"' | '\'' | '\\'))
            .filter(|&c| std_escapes(c) && !is_escaped(c))
            .map(u32::from)
            .collect();
        assert!(
            shown_by_us.is_empty(),
            "this toolchain's str::escape_debug escapes {} char(s) the frozen set shows as \
             written (first 32: {:X?}); decide whether the set must widen (regenerate the tables)",
            shown_by_us.len(),
            &shown_by_us[..shown_by_us.len().min(32)]
        );
    }

    /// T4b (report): chars we escape that this toolchain's std prints. Expected 0 on rustc 1.99.1
    /// and 267 Invisible (the remaining Default_Ignorable code points) on 1.96.0. Fails only if
    /// such a char is a Control or PrivateUse one, which would mean std changed something odd.
    #[test]
    fn report_where_we_are_stricter_than_std() {
        let mut by_class = [0usize; 4];
        for c in all_chars().filter(|&c| !matches!(c, '"' | '\'' | '\\')) {
            if let Some(class) = escape_class(c) {
                if !std_escapes(c) {
                    let k = TABLES.iter().position(|t| t.2 == class).expect("class");
                    by_class[k] += 1;
                    assert!(
                        matches!(class, EscapeClass::Invisible | EscapeClass::Unassigned),
                        "std prints U+{:04X} ({class:?}) as written",
                        u32::from(c)
                    );
                }
            }
        }
        eprintln!(
            "text_escape: escaped here but printed by this toolchain's std: Control {}, \
             Invisible {}, PrivateUse {}, Unassigned {}",
            by_class[0], by_class[1], by_class[2], by_class[3]
        );
    }

    /// T5b (report): `push_escape` against the building toolchain's `char::escape_debug` for
    /// every escaped char. Ours is the contract, so a difference is reported, not failed.
    #[test]
    fn report_rendering_differences_from_std() {
        let mut differ = 0usize;
        let mut first = Vec::new();
        for c in all_chars().filter(|&c| is_escaped(c)) {
            let mut ours = String::new();
            push_escape(&mut ours, c);
            if ours != c.escape_debug().to_string() {
                differ += 1;
                if first.len() < 8 {
                    first.push(u32::from(c));
                }
            }
        }
        eprintln!(
            "text_escape: rendering differs from this toolchain's char::escape_debug for {differ} \
             escaped char(s) (first: {first:X?})"
        );
    }

    /// L-94: NO std Unicode table on the decision or rendering path. The non-test part of this
    /// file must not call a std char classifier or escaper, nor format anything with `Debug`.
    /// (On rustc 1.99.1 a return to std's probe would be invisible to every other test here,
    /// because std's set equals the frozen one there.)
    #[test]
    fn the_decision_path_consults_no_std_unicode_table() {
        let src = include_str!("text_escape.rs");
        let code = &src[..src
            .find("\n#[cfg(test)]\nmod tests")
            .expect("the test module marker")];
        let forbidden = [
            "escape_debug",
            "escape_default",
            "escape_unicode",
            "is_control",
            "is_whitespace",
            "is_alphabetic",
            "is_alphanumeric",
            "is_numeric",
            "is_lowercase",
            "is_uppercase",
            "to_lowercase",
            "to_uppercase",
            ":?}",
            "{:?",
        ];
        for (n, line) in code.lines().enumerate() {
            let t = line.trim_start();
            if t.starts_with("//") {
                continue;
            }
            for f in forbidden {
                assert!(!line.contains(f), "line {}: `{f}` in `{line}`", n + 1);
            }
        }
    }
}
