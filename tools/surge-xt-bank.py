#!/usr/bin/env python3
"""Package the pinned, complete upstream FXPs; shared by Android and native audio QA."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import struct

REVISION = "5a4828a33ac4b2e2b45671b34e9e0e3f0c614765"


def chunk(path):
    data = path.read_bytes()
    if (len(data) < 92 or data[:4] != b"CcnK" or data[8:12] != b"FPCh"
            or data[16:20] != b"cjs3"):
        raise ValueError(f"Invalid official Surge FXP header: {path}")
    size = int.from_bytes(data[56:60], "big")
    if size != len(data) - 60:
        raise ValueError(f"Truncated official Surge FXP: {path}")
    payload = data[60:]
    if payload[:4] != b"sub3":
        raise ValueError(f"Missing original Surge patch chunk: {path}")
    xml_size, *wt_sizes = struct.unpack_from("<7I", payload, 4)
    pos = 32 + xml_size
    if xml_size < 100 or pos > len(payload):
        raise ValueError(f"Invalid patch XML length: {path}")
    # Do not parse/rewrite the XML. TinyXML (the real loader) supports legacy
    # character references that Python's strict XML parser rejects.
    for wt_size in wt_sizes:
        if wt_size:
            if wt_size < 12 or pos + wt_size > len(payload):
                raise ValueError(f"Truncated embedded wavetable: {path}")
            samples, tables, flags = struct.unpack_from("<IHH", payload, pos + 4)
            if samples * tables * (2 if flags & 4 else 4) + 12 != wt_size:
                raise ValueError(f"Invalid embedded wavetable size: {path}")
            pos += wt_size
    return data, payload, wt_sizes


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    parser.add_argument("generated", type=Path)
    parser.add_argument("catalog", type=Path)
    parser.add_argument("--assets", type=Path)
    args = parser.parse_args()
    root = args.source / "resources/data/patches_factory"
    files = args.catalog.read_text().splitlines()
    actual = sorted(p.relative_to(root).as_posix() for p in root.rglob("*.fxp"))
    if actual != files or len(files) != 641 or len(set(files)) != 641:
        raise ValueError("Pinned official factory catalog does not match all 641 FXPs")
    out = args.generated.resolve()
    out.mkdir(parents=True, exist_ok=True)
    blob, records, report = bytearray(), [], []
    for index, rel in enumerate(files, 1):
        data, payload, wt_sizes = chunk(root / rel)
        records.append(f"    {{{len(blob)}u, {len(payload)}u, {json.dumps(rel)}}}")
        report.append(dict(index=index, path=rel, fxp_bytes=len(data),
                           fxp_sha256=hashlib.sha256(data).hexdigest(),
                           chunk_sha256=hashlib.sha256(payload).hexdigest(),
                           embedded_wavetables=sum(n != 0 for n in wt_sizes)))
        blob.extend(payload)
    (out / "surge_factory_patches.bin").write_bytes(blob)
    # Absolute path is consumed by the assembler, not by a shell.
    bank = "\n".join([
        "#pragma once", "#include <cstdint>",
        "struct HoneydewSurgeFactoryRecord { uint32_t offset; uint32_t size; const char *path; };",
        f"static constexpr int honeydew_surge_factory_count = {len(files)};",
        "static constexpr HoneydewSurgeFactoryRecord honeydew_surge_factory[] = {",
        ",\n".join(records), "};",
        'extern "C" const unsigned char honeydew_surge_factory_blob_start[];',
        'asm(R"surge(', '.pushsection .rodata', '.balign 16',
        '.global honeydew_surge_factory_blob_start',
        '.hidden honeydew_surge_factory_blob_start',
        'honeydew_surge_factory_blob_start:',
        f'.incbin "{out / "surge_factory_patches.bin"}"',
        '.popsection', ')surge");', "",
    ])
    (out / "honeydew_surge_factory_bank.h").write_text(bank)
    _, init, _ = chunk(root / "Templates/Init Saw.fxp")
    (out / "honeydew_surge_init_patch.h").write_text(
        "#pragma once\nstatic const unsigned char honeydew_surge_init_data[] = {\n" +
        "\n".join("    " + ", ".join(f"0x{v:02x}" for v in init[i:i+16]) + ","
                  for i in range(0, len(init), 16)) + "\n};\n")
    (out / "factory-bank.json").write_text(json.dumps(
        dict(revision=REVISION, patches=report), indent=2) + "\n")
    if args.assets:
        assets = args.assets
        if assets.exists():
            shutil.rmtree(assets)
        assets.mkdir(parents=True)
        # Patch data include their oscillator tables and effect data. Also ship
        # the original stock resources, including Window WT in surge-common's
        # binary resources, with no dependency on a desktop installation.
        for directory in ["patches_factory", "wavetables", "impulses_factory", "tuning_library"]:
            shutil.copytree(args.source / "resources/data" / directory, assets / directory)
        shutil.copyfile(args.source / "LICENSE", assets / "Surge-XT-LICENSE.txt")
        shutil.copyfile(out / "factory-bank.json", assets / "factory-bank.json")
        entries = []
        for path in sorted(p for p in assets.rglob("*") if p.is_file()):
            entries.append(hashlib.sha256(path.read_bytes()).hexdigest() + "\t" +
                           path.relative_to(assets).as_posix())
        (assets / "factory-manifest.tsv").write_text("\n".join(entries) + "\n")
    print(f"Verified {len(files)} authentic FXPs, {len(blob)} chunk bytes, "
          f"{sum(r['embedded_wavetables'] for r in report)} embedded oscillator tables")


if __name__ == "__main__":
    main()
