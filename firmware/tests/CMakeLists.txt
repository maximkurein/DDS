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
if(MINGW)
    target_compile_definitions(dds_core_tests PRIVATE __USE_MINGW_ANSI_STDIO=1)
endif()

if(MSVC)
    target_compile_options(dds_core_tests PRIVATE /W4)
else()
    target_compile_options(dds_core_tests PRIVATE -Wall -Wextra -Wshadow -Wconversion)
endif()

add_test(NAME dds_core_tests COMMAND dds_core_tests)

# Production application/drivers with deterministic hardware and time substitutes.
add_executable(dds_firmware_tests
    test_firmware.c
    ../src/app/app.c
    ../src/core/debounce.c
    ../src/core/freq.c
    ../src/drivers/board.c
    ../src/drivers/pl8t.c
    ../src/drivers/osccal.c)
target_include_directories(dds_firmware_tests PRIVATE fakes ../src ../config)
target_compile_definitions(dds_firmware_tests PRIVATE F_CPU=8000000UL DDS_LOG=0 DDS_WDT=0)
target_compile_features(dds_firmware_tests PRIVATE c_std_11)
if(MSVC)
    target_compile_options(dds_firmware_tests PRIVATE /W4)
else()
    target_compile_options(dds_firmware_tests PRIVATE -Wall -Wextra -Wshadow -Wconversion)
endif()
add_test(NAME dds_firmware_tests COMMAND dds_firmware_tests)
