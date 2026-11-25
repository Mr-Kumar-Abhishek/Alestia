#!/usr/bin/env python3
"""Convert Celestia's truncated VSOP87 C++ tables into `vsop87.bin`.

Usage: python gen_vsop87.py <path/to/vsop87.cpp> [output.bin]

Binary layout (all little-endian):
  for each body in BODIES (9):
    for each of 3 variables (L,B,R for planets; X,Y,Z for the Sun):
      u32 series_count
      for each series (power of t, ascending):
        u32 term_count
        term_count * (f64 A, f64 B, f64 C)
"""
import re
import struct
import sys

BODIES = [
    ("mercury", "LBR"),
    ("venus", "LBR"),
    ("earth", "LBR"),
    ("mars", "LBR"),
    ("jupiter", "LBR"),
    ("saturn", "LBR"),
    ("uranus", "LBR"),
    ("neptune", "LBR"),
    ("sun", "XYZ"),
]

BLOCK = re.compile(
    r"constexpr std::array<VSOPTerm,\s*(\d+)>\s+(\w+)\s*\{(.*?)\};", re.S
)
TERM = re.compile(r"VSOPTerm\s*\{\s*([^,]+),\s*([^,]+),\s*([^}]+?)\s*\}")


def main() -> None:
    src = open(sys.argv[1], encoding="utf-8").read()
    out_path = sys.argv[2] if len(sys.argv) > 2 else "vsop87.bin"
    tables = {}
    for count, name, body in BLOCK.findall(src):
        terms = [tuple(float(x) for x in m) for m in TERM.findall(body)]
        assert len(terms) == int(count), (name, len(terms), count)
        tables[name] = terms

    out = bytearray()
    total = 0
    for body, variables in BODIES:
        for var in variables:
            series = []
            index = 0
            while f"{body}_{var}{index}" in tables:
                series.append(tables[f"{body}_{var}{index}"])
                index += 1
            out += struct.pack("<I", len(series))
            for terms in series:
                out += struct.pack("<I", len(terms))
                for term in terms:
                    out += struct.pack("<ddd", *term)
                total += len(terms)
    with open(out_path, "wb") as handle:
        handle.write(out)
    print(f"wrote {out_path}: {len(out)} bytes, {total} terms, {len(tables)} series")


if __name__ == "__main__":
    main()
