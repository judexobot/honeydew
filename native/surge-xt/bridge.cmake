# Included at the end of the ORIGINAL Surge XT root CMakeLists.txt, after add_subdirectory(src).
# SURGE_SKIP_JUCE_FOR_RACK removes the JUCE application and plugin wrapper.
# Only the original surge-common DSP is linked into the native Android bridge.
if(NOT TARGET surge-common)
  message(FATAL_ERROR "Surge XT core target (surge-common) is missing")
endif()
if(NOT DEFINED HONEYDEW_SURGE_DIR)
  message(FATAL_ERROR "Set -DHONEYDEW_SURGE_DIR=<honeydew checkout>")
endif()
add_library(honeydew_surge_xt SHARED
  "${HONEYDEW_SURGE_DIR}/native/surge-xt/surge_honeydew.cpp"
)
target_include_directories(honeydew_surge_xt PRIVATE
  "${SURGE_SOURCE_DIR}/src/common"
  "${CMAKE_BINARY_DIR}/generated"
)
target_link_libraries(honeydew_surge_xt PRIVATE surge-common)
option(HONEYDEW_SURGE_APP_PATHS "Test the Android private-path adapter on native Linux" OFF)
if(HONEYDEW_SURGE_APP_PATHS)
  target_compile_definitions(sst-plugininfra PRIVATE HONEYDEW_SURGE_APP_PATHS=1)
endif()
# Android's linker can otherwise emit an ELF with unresolved symbols,
# causing dlopen(RTLD_NOW) to fail only when the user opens an instrument.
# Diagnose those missing dependencies during GitHub Actions instead.
if(ANDROID)
  target_link_options(honeydew_surge_xt PRIVATE "-Wl,--no-undefined" "-Wl,-z,max-page-size=16384")
endif()
target_compile_features(honeydew_surge_xt PRIVATE cxx_std_20)
set_target_properties(honeydew_surge_xt PROPERTIES
  OUTPUT_NAME "honeydew_surge_xt"
  CXX_VISIBILITY_PRESET hidden
  VISIBILITY_INLINES_HIDDEN YES
)
# The ABI and JNI entry points carry explicit default visibility. Keep all
# implementation/template symbols hidden, including those in this bridge.

option(HONEYDEW_SURGE_TESTS "Build audio tests using the real Surge engine" OFF)
if(HONEYDEW_SURGE_TESTS)
  enable_testing()
  add_executable(honeydew_surge_audio_test
    "${HONEYDEW_SURGE_DIR}/native/surge-xt/audio_test.cpp")
  target_include_directories(honeydew_surge_audio_test PRIVATE "${CMAKE_BINARY_DIR}/generated")
  target_link_libraries(honeydew_surge_audio_test PRIVATE surge-common)
endif()
