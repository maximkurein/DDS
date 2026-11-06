# Unit-тесты чистой логики (src/core) на ПК: gcc, clang или MSVC.
#   cmake --preset host-tests && cmake --build --preset host-tests && ctest --preset host-tests

add_executable(dds_core_tests
    test_main.c
    test_freq.c
    test_dip.c
    test_debounce.c
    ../src/core/freq.c
    ../src/core/debounce.c)

target_include_directories(dds_core_tests PRIVATE ../src)
target_compile_features(dds_core_tests PRIVATE c_std_11)

if(MSVC)
    target_compile_options(dds_core_tests PRIVATE /W4)
else()
    target_compile_options(dds_core_tests PRIVATE -Wall -Wextra -Wshadow -Wconversion)
endif()

add_test(NAME dds_core_tests COMMAND dds_core_tests)
