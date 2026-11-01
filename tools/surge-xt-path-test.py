#!/usr/bin/env python3
"""Exercise the actual shared library with Android-like path restrictions."""
import ctypes
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile


def child(library, factory, private):
    lib = ctypes.CDLL(str(library))
    lib.honeydew_surge_set_data_path.argtypes = [ctypes.c_char_p, ctypes.c_char_p]
    lib.honeydew_surge_create.argtypes = [ctypes.c_float]
    lib.honeydew_surge_create.restype = ctypes.c_void_p
    lib.honeydew_surge_destroy.argtypes = [ctypes.c_void_p]
    lib.honeydew_surge_probe.argtypes = [ctypes.c_float]
    lib.honeydew_surge_probe.restype = ctypes.c_float
    lib.honeydew_surge_last_error.restype = ctypes.c_char_p
    if not lib.honeydew_surge_set_data_path(os.fsencode(factory), os.fsencode(private)):
        raise RuntimeError(lib.honeydew_surge_last_error().decode())
    # dladdr still returns this mapped library location after permissions are
    # removed. Desktop portable-folder discovery must not walk that location.
    os.chmod(library.parent, 0)
    try:
        assert not os.access(library.parent, os.X_OK), "Run path QA as an unprivileged user"
        handle = lib.honeydew_surge_create(48000)
        if not handle:
            raise RuntimeError(lib.honeydew_surge_last_error().decode())
        lib.honeydew_surge_destroy(handle)
        peak = lib.honeydew_surge_probe(48000)
        assert peak > 0.00001, lib.honeydew_surge_last_error().decode()
        print(json.dumps(dict(engine_created=True, init_saw_peak=peak,
                              login_home_present="HOME" in os.environ,
                              library_parent_searchable=False)))
    finally:
        os.chmod(library.parent, 0o700)


def main():
    if len(sys.argv) == 5 and sys.argv[1] == "--child":
        child(*(Path(p) for p in sys.argv[2:]))
        return
    if len(sys.argv) != 4:
        raise SystemExit("usage: surge-xt-path-test.py <shared-library> <factory-data> <evidence-dir>")
    library, factory, evidence = (Path(p).resolve() for p in sys.argv[1:])
    evidence.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="surge-private-paths-") as temp:
        root = Path(temp)
        locked = root / "apk-library-parent"
        locked.mkdir()
        copy = locked / library.name
        shutil.copyfile(library, copy)
        process = subprocess.run([sys.executable, __file__, "--child", str(copy),
                                  str(factory), str(root / "private-user")],
                                 env={k: v for k, v in os.environ.items() if k != "HOME"},
                                 text=True, capture_output=True)
        if process.returncode:
            raise RuntimeError(process.stdout + process.stderr)
        result = json.loads(process.stdout.splitlines()[-1])
        assert result["engine_created"] and not result["login_home_present"]
        (evidence / "android-private-paths.json").write_text(json.dumps(result, indent=2) + "\n")
        print("PASS: original Init Saw PCM without HOME or access to the APK library's parent")


if __name__ == "__main__":
    main()
