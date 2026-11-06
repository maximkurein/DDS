# Симуляция прошивки в simavr (только Linux / WSL).
# Нужны пакеты: simavr libsimavr-dev libelf-dev pkg-config.
# ELF берётся из сборки avr-debug (сначала: cmake --build --preset avr-debug).

find_package(PkgConfig REQUIRED)
pkg_check_modules(SIMAVR REQUIRED IMPORTED_TARGET simavr)

set(DDS_SIM_ELF "${PROJECT_SOURCE_DIR}/build/avr-debug/dds_fw.elf"
    CACHE FILEPATH "ELF прошивки для симуляции")

add_executable(sim_dds sim_dds.c)
target_include_directories(sim_dds PRIVATE ../src)
target_link_libraries(sim_dds PRIVATE PkgConfig::SIMAVR)

add_test(NAME sim_dds COMMAND sim_dds "${DDS_SIM_ELF}")
add_test(NAME sim_dds_empty_cal COMMAND sim_dds "${DDS_SIM_ELF}" empty)
add_test(NAME sim_dds_invalid_cal COMMAND sim_dds "${DDS_SIM_ELF}" invalid)
