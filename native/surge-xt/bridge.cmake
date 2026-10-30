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
target_include_directories(honeydew_surge_xt PRIVATE "${SURGE_SOURCE_DIR}/src/common")
target_link_libraries(honeydew_surge_xt PRIVATE surge-common)
target_compile_features(honeydew_surge_xt PRIVATE cxx_std_20)
set_target_properties(honeydew_surge_xt PROPERTIES
  OUTPUT_NAME "honeydew_surge_xt"
  CXX_VISIBILITY_PRESET hidden
  VISIBILITY_INLINES_HIDDEN YES
)
# The C symbols are exported explicitly; Surge's C++ internals stay hidden.
set_source_files_properties("${HONEYDEW_SURGE_DIR}/native/surge-xt/surge_honeydew.cpp"
  PROPERTIES COMPILE_OPTIONS "-fvisibility=default")
