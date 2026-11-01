#!/usr/bin/env python3
"""Apply idempotent platform/build adaptations, never modify Surge's DSP algorithms."""
from pathlib import Path
import sys

source, honeydew = (Path(p).resolve() for p in sys.argv[1:])


def replace(path, old, new):
    text = path.read_text()
    if new in text:
        return
    if text.count(old) != 1:
        raise ValueError(f"Pinned source changed: {path}: {old[:80]}")
    path.write_text(text.replace(old, new, 1))


# Android apps do not have a Unix login HOME. Do not mutate process-wide
# environment variables: route Surge's platform paths into our private files.
paths = source / "libs/sst/sst-plugininfra/src/paths_linux.cpp"
replace(paths, "namespace sst\n{", """#if defined(__ANDROID__) || defined(HONEYDEW_SURGE_APP_PATHS)
extern "C" const char *honeydew_surge_private_home() noexcept;
#endif

namespace sst
{""")
replace(paths, '    const char *const path = std::getenv("HOME");', """#if defined(__ANDROID__) || defined(HONEYDEW_SURGE_APP_PATHS)
    const char *const path = honeydew_surge_private_home();
#else
    const char *const path = std::getenv("HOME");
#endif""")
replace(paths, 'throw std::runtime_error{"The environment variable HOME is unset or empty"};',
        'throw std::runtime_error{"Surge private user path was not initialized by Honeydew"};')

# A private HOME alone is insufficient: desktop portable-mode discovery walks
# the APK library's parents, some of which an Android app cannot stat. Adapt
# only storage-path initialization, keeping all synthesis/table code intact.
storage = source / "src/common/SurgeStorage.cpp"
replace(storage, "namespace mech = sst::basic_blocks::mechanics;", """#if defined(__ANDROID__) || defined(HONEYDEW_SURGE_APP_PATHS)
extern "C" const char *honeydew_surge_factory_directory() noexcept;
#endif

namespace mech = sst::basic_blocks::mechanics;""")
replace(storage, """    localAppDataPath = sst::plugininfra::paths::bestLibrarySharedVendorFolderPathFor(
        "Surge Synth Team", "Surge XT", true);
    userDataPath = getOverridenUserPath();""", """#if defined(__ANDROID__) || defined(HONEYDEW_SURGE_APP_PATHS)
    const auto privateHome = sst::plugininfra::paths::homePath();
    localAppDataPath = privateHome / "application";
    userDataPath = privateHome / "userdata";
#else
    localAppDataPath = sst::plugininfra::paths::bestLibrarySharedVendorFolderPathFor(
        "Surge Synth Team", "Surge XT", true);
    userDataPath = getOverridenUserPath();
#endif""")
replace(storage, """    bool isUserPortable{false};

#if MAC""", """    bool isUserPortable{false};

#if defined(__ANDROID__) || defined(HONEYDEW_SURGE_APP_PATHS)
    const auto factoryDirectory = honeydew_surge_factory_directory();
    if (!factoryDirectory || !factoryDirectory[0])
        throw std::runtime_error("Honeydew did not initialize the Surge factory directory");
    datapath = string_to_path(factoryDirectory);
#elif MAC""")

misc = source / "libs/sst/sst-plugininfra/src/misc_linux.cpp"
replace(misc, "#include <execinfo.h>", "#if !defined(__ANDROID__)\n#include <execinfo.h>\n#endif")
replace(misc, "std::string stackTraceToString(int depth)\n{\n    void *callstack[128];", """std::string stackTraceToString(int depth)
{
#if defined(__ANDROID__)
    (void)depth;
    return "Native stack trace unavailable with Android Bionic";
#else
    void *callstack[128];""")
replace(misc, "    free(strs);\n    return oss.str();\n}",
        "    free(strs);\n    return oss.str();\n#endif\n}")

common = source / "src/common/CMakeLists.txt"
text = common.read_text()
text = text.replace("      pthread\n", "      $<$<NOT:$<PLATFORM_ID:Android>>:pthread>\n")
common.write_text(text)
root = source / "CMakeLists.txt"
addon = f'\ninclude("{honeydew}/native/surge-xt/bridge.cmake")\n'
text = root.read_text()
if addon not in text:
    root.write_text(text + addon)
