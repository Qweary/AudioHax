#!/usr/bin/env python3
"""Generate the escaped-set tables in src/text_escape.rs from the Unicode Character Database.

S125b (lead ruling L-94): the set of characters AudioHax escapes when it prints untrusted text
(`text_escape::escape_controls`) is FROZEN as range tables generated here, so the set a binary
escapes never depends on the rustc that built it. This script is the only thing that may write
the block between the BEGIN/END GENERATED markers in src/text_escape.rs.

Inputs: two files from the Unicode Character Database, version 17.0.0, checked against the
sha256 values pinned below (the script refuses any other bytes):

    https://www.unicode.org/Public/17.0.0/ucd/extracted/DerivedGeneralCategory.txt
    https://www.unicode.org/Public/17.0.0/ucd/DerivedCoreProperties.txt

The UCD files are NOT committed. Download them yourself (any directory), then:

    python3 -I tools/gen_text_escape_tables.py --ucd DIR            # rewrite the block
    python3 -I tools/gen_text_escape_tables.py --ucd DIR --check    # exit 1 if the block differs

Classes (all from UCD 17.0.0; disjoint; a scalar value is in at most one):

    Control    = General_Category Cc
    Invisible  = (Cf | Zs minus U+0020 | Zl | Zp | Default_Ignorable_Code_Point) minus Control
    PrivateUse = Co minus Invisible
    Unassigned = Cn minus Invisible minus PrivateUse

The script also prints the per-class counts and the FNV-1a-64 digest that the frozen-set test
(`text_escape::tests::escape_set_is_frozen`) asserts, so the test constants come from the same run.
Python 3 standard library only.
"""

import argparse
import hashlib
import os
import sys

UNICODE_VERSION = (17, 0, 0)

# sha256 of the published UCD 17.0.0 files (fetched from unicode.org, S125b; equal to Debian's
# unicode-data 17.0.0 copies).
PINNED = {
    "DerivedGeneralCategory.txt": "d62e5bab70ca74f099343f71224fa051cb1fdd61a1ab45c0488c44cfc0b6102e",
    "DerivedCoreProperties.txt": "24c7fed1195c482faaefd5c1e7eb821c5ee1fb6de07ecdbaa64b56a99da22c08",
}

BEGIN = "// BEGIN GENERATED TABLES"
END = "// END GENERATED TABLES"

MAX_SCALAR = 0x10FFFF


def find_input(ucd_dir, name):
    for cand in (os.path.join(ucd_dir, name), os.path.join(ucd_dir, "extracted", name)):
        if os.path.isfile(cand):
            return cand
    sys.exit(f"error: {name} not found under {ucd_dir} (or its extracted/ subdirectory)")


def read_checked(path, name):
    with open(path, "rb") as f:
        data = f.read()
    got = hashlib.sha256(data).hexdigest()
    if got != PINNED[name]:
        sys.exit(f"error: {path}: sha256 {got} is not the pinned UCD 17.0.0 value {PINNED[name]}")
    text = data.decode("utf-8")
    want = "# %s-%d.%d.%d.txt" % ((name[:-4],) + UNICODE_VERSION)
    if not text.startswith(want):
        sys.exit(f"error: {path}: first line is not {want!r}")
    return text


def parse(text):
    """Yield (first, last, value) for each data line `XXXX[..YYYY] ; Value [; ...] # ...`."""
    for line in text.splitlines():
        line = line.split("#", 1)[0].strip()
        if not line:
            continue
        fields = [f.strip() for f in line.split(";")]
        cps = fields[0]
        if ".." in cps:
            a, b = cps.split("..")
            first, last = int(a, 16), int(b, 16)
        else:
            first = last = int(cps, 16)
        yield first, last, fields[1]


def ranges_of(cps):
    out = []
    for cp in sorted(cps):
        if out and out[-1][1] + 1 == cp:
            out[-1][1] = cp
        else:
            out.append([cp, cp])
    return [tuple(r) for r in out]


def fnv1a64(scalars):
    h = 0xCBF29CE484222325
    for cp in scalars:
        for byte in cp.to_bytes(4, "little"):
            h ^= byte
            h = (h * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
    return h


def classify(gc_text, dcp_text):
    gc = {}
    for first, last, value in parse(gc_text):
        for cp in range(first, last + 1):
            gc[cp] = value
    # DerivedGeneralCategory lists every code point except Cn ones, which take the @missing default.
    for cp in range(MAX_SCALAR + 1):
        gc.setdefault(cp, "Cn")
    dicp = set()
    for first, last, value in parse(dcp_text):
        if value == "Default_Ignorable_Code_Point":
            dicp.update(range(first, last + 1))
    if not dicp:
        sys.exit("error: no Default_Ignorable_Code_Point lines found")

    def cat(*names):
        return {cp for cp, v in gc.items() if v in names}

    control = cat("Cc")
    invisible = (cat("Cf", "Zl", "Zp") | (cat("Zs") - {0x20}) | dicp) - control
    private_use = cat("Co") - invisible
    unassigned = cat("Cn") - invisible - private_use
    surrogates = cat("Cs")
    for name, s in (("Control", control), ("Invisible", invisible),
                    ("PrivateUse", private_use), ("Unassigned", unassigned)):
        if s & surrogates:
            sys.exit(f"error: {name} contains a surrogate")
    return control, invisible, private_use, unassigned


def table_rust(name, doc, rngs):
    lines = [f"/// {d}" if d else "///" for d in doc]
    # One row per line, always (reviewable diffs on regeneration); rustfmt must not re-flow it.
    lines.append("#[rustfmt::skip]")
    lines.append(f"pub const {name}: &[ScalarRange] = &[")
    for a, b in rngs:
        lines.append(f"    (0x{a:04X}, 0x{b:04X}),")
    lines.append("];")
    return "\n".join(lines)


def generate_block(classes):
    control, invisible, private_use, unassigned = classes
    v = "%d.%d.%d" % UNICODE_VERSION
    parts = [
        BEGIN + " — written by tools/gen_text_escape_tables.py from UCD " + v + "; DO NOT EDIT BY HAND.",
        "// Inputs (sha256): DerivedGeneralCategory.txt " + PINNED["DerivedGeneralCategory.txt"] + ",",
        "// DerivedCoreProperties.txt " + PINNED["DerivedCoreProperties.txt"] + ".",
        "",
        "/// The Unicode version the tables below were generated from.",
        "pub const TABLES_UNICODE_VERSION: (u8, u8, u8) = (%d, %d, %d);" % UNICODE_VERSION,
        "",
        table_rust("CONTROL", ["[`EscapeClass::Control`]: General_Category Cc (%d code points)." % len(control)],
                   ranges_of(control)),
        "",
        table_rust("INVISIBLE",
                   ["[`EscapeClass::Invisible`]: Cf, Zs other than U+0020, Zl, Zp and every",
                    "Default_Ignorable_Code_Point, less Cc (%d code points)." % len(invisible)],
                   ranges_of(invisible)),
        "",
        table_rust("PRIVATE_USE", ["[`EscapeClass::PrivateUse`]: Co, less Invisible (%d code points)." % len(private_use)],
                   ranges_of(private_use)),
        "",
        table_rust("UNASSIGNED",
                   ["[`EscapeClass::Unassigned`]: Cn in Unicode %s, less Invisible and PrivateUse" % v,
                    "(%d code points)." % len(unassigned)],
                   ranges_of(unassigned)),
        END,
    ]
    return "\n".join(parts) + "\n"


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--ucd", required=True, help="directory holding the two UCD 17.0.0 files")
    ap.add_argument("--target", default=os.path.join(os.path.dirname(os.path.abspath(__file__)),
                                                     os.pardir, "src", "text_escape.rs"))
    ap.add_argument("--check", action="store_true", help="do not write; exit 1 if the block differs")
    args = ap.parse_args()

    gc_text = read_checked(find_input(args.ucd, "DerivedGeneralCategory.txt"), "DerivedGeneralCategory.txt")
    dcp_text = read_checked(find_input(args.ucd, "DerivedCoreProperties.txt"), "DerivedCoreProperties.txt")
    classes = classify(gc_text, dcp_text)
    block = generate_block(classes)

    control, invisible, private_use, unassigned = classes
    escaped = sorted(control | invisible | private_use | unassigned)
    for name, s in (("Control", control), ("Invisible", invisible),
                    ("PrivateUse", private_use), ("Unassigned", unassigned)):
        print(f"{name:<11} {len(s):>7} code points, {len(ranges_of(s)):>4} ranges")
    print(f"{'total':<11} {len(escaped):>7} code points")
    print(f"fnv1a64     0x{fnv1a64(escaped):016x}  (little-endian u32 of every escaped scalar, ascending)")

    with open(args.target, encoding="utf-8") as f:
        src = f.read()
    i, j = src.find(BEGIN), src.find(END)
    if i < 0 or j < 0 or j < i or src.count(BEGIN) != 1 or src.count(END) != 1:
        sys.exit(f"error: {args.target}: expected exactly one BEGIN/END GENERATED TABLES pair")
    j = src.index("\n", j) + 1
    new = src[:i] + block + src[j:]
    if args.check:
        if new != src:
            print(f"{args.target}: generated block is NOT what UCD {'.'.join(map(str, UNICODE_VERSION))} gives")
            return 1
        print(f"{args.target}: generated block matches")
        return 0
    if new != src:
        with open(args.target, "w", encoding="utf-8", newline="\n") as f:
            f.write(new)
        print(f"{args.target}: block rewritten")
    else:
        print(f"{args.target}: block already current")
    return 0


if __name__ == "__main__":
    sys.exit(main())
