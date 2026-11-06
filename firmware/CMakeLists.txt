# Прошивка контроллера D2 (1887ВЕ7Т) синтезатора DDS 48–52 МГц.
#
# Пресеты (CMakePresets.json):
#   avr-debug    — прошивка с отладочной информацией и UART-логом;
#   avr-release  — рабочая прошивка без лога;
#   host-tests   — unit-тесты чистой логики на ПК (CTest);
#   sim          — то же плюс прогон прошивки в simavr с моделью 1508ПЛ8Т (Linux/WSL).
#
# Цели программирования (avrdude, профиль 1887ВЕ7Т):
#   flash, flash_clean, verify, probe, erase, fuses_read, fuses_write,
#   osccal, backup. Пример: cmake --build --preset avr-debug --target flash

cmake_minimum_required(VERSION 3.21)
project(dds_fw
    VERSION 1.0.1
    DESCRIPTION "DDS 48-52 MHz controller firmware (1887VE7T + 1508PL8T)"
    LANGUAGES C)

option(DDS_HOST_TESTS "Собрать unit-тесты для ПК вместо прошивки" OFF)
option(DDS_SIM "Вместе с тестами собрать симуляцию simavr (Linux/WSL)" OFF)

if(DDS_HOST_TESTS)
    enable_testing()
    add_subdirectory(tests)
    if(DDS_SIM)
        add_subdirectory(sim)
    endif()
    return()
endif()

if(NOT CMAKE_SYSTEM_PROCESSOR STREQUAL "avr")
    message(FATAL_ERROR
        "Прошивку собирают пресетом avr-debug или avr-release "
        "(toolchain cmake/avr-gcc.cmake). Для тестов на ПК — пресет host-tests.")
endif()

# ---------------------------------------------------------------------------
# Параметры цели
# ---------------------------------------------------------------------------
set(DDS_MCU   "atmega128" CACHE STRING "Модель для -mmcu: 1887ВЕ7Т собирается как ATmega128")
set(DDS_F_CPU "8000000"   CACHE STRING "Частота ядра, Гц (внутренний RC 8 МГц, CKSEL=0100)")
option(DDS_LOG "Отладочный вывод в USART0 (PE1 / площадка X1.1)" OFF)
option(DDS_WDT "Сторожевой таймер (в отладочной сборке выключен, чтобы не мешал остановкам)" ON)

# Фьюзы 1887ВЕ7Т (0 = запрограммирован). Расшифровка — README.md, раздел «Фьюзы».
#   Low  0x24: BODLEVEL=0 (4,0 В), BODEN=0, SUT=10, CKSEL=0100 (RC 8 МГц)
#   High 0x11: OCDEN=0 (отладка), JTAGEN=0, SPIEN=0, CKOPT=1, EESAVE=0, BOOTSZ=00, BOOTRST=1
#   High 0x91: то же, но OCDEN=1 (для серийной прошивки)
#   Ext  0xFF: M103C=1 (режим ATmega128, НЕ ATmega103!), WDTON=1
set(DDS_FUSE_LOW  "0x24" CACHE STRING "Low fuse")
set(DDS_FUSE_HIGH "0x11" CACHE STRING "High fuse (0x11 — отладка, 0x91 — серия)")
set(DDS_FUSE_EXT  "0xFF" CACHE STRING "Extended fuse")

# ---------------------------------------------------------------------------
# Версия: project(VERSION) + git describe
# ---------------------------------------------------------------------------
set(DDS_GIT_REV "nogit")
find_package(Git QUIET)
if(GIT_FOUND)
    execute_process(
        COMMAND "${GIT_EXECUTABLE}" describe --always --dirty --abbrev=8
        WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}"
        OUTPUT_VARIABLE _git_rev OUTPUT_STRIP_TRAILING_WHITESPACE
        RESULT_VARIABLE _git_rc ERROR_QUIET)
    if(_git_rc EQUAL 0 AND _git_rev)
        set(DDS_GIT_REV "${_git_rev}")
    endif()
endif()
configure_file(cmake/fw_version.h.in "${CMAKE_CURRENT_BINARY_DIR}/generated/fw_version.h" @ONLY)

# ---------------------------------------------------------------------------
# Прошивка
# ---------------------------------------------------------------------------
add_executable(dds_fw
    src/main.c
    src/app/app.c
    src/core/debounce.c
    src/core/freq.c
    src/drivers/board.c
    src/drivers/osccal.c
    src/drivers/pl8t.c
    src/drivers/spi.c
    src/drivers/systick.c
    src/drivers/uart_log.c)

target_include_directories(dds_fw PRIVATE
    src
    config
    "${CMAKE_CURRENT_BINARY_DIR}/generated")

target_compile_definitions(dds_fw PRIVATE
    F_CPU=${DDS_F_CPU}UL
    DDS_LOG=$<BOOL:${DDS_LOG}>
    DDS_WDT=$<BOOL:${DDS_WDT}>)

target_compile_features(dds_fw PRIVATE c_std_11)
set_target_properties(dds_fw PROPERTIES C_EXTENSIONS ON)   # gnu11: ISR(), PSTR(), __attribute__

target_compile_options(dds_fw PRIVATE
    -mmcu=${DDS_MCU}
    -Wall -Wextra -Wshadow -Wundef
    -Wstrict-prototypes -Wmissing-prototypes
    -Werror=implicit-function-declaration
    -funsigned-char -funsigned-bitfields -fshort-enums -fno-common
    -ffunction-sections -fdata-sections)

# avr-gcc 12+ ошибочно считает обращения к регистрам по адресам < 4096
# выходом за границы массива (-Warray-bounds). Штатное решение — min-pagesize=0.
if(CMAKE_C_COMPILER_VERSION VERSION_GREATER_EQUAL 12)
    target_compile_options(dds_fw PRIVATE --param=min-pagesize=0)
endif()

target_link_options(dds_fw PRIVATE
    -mmcu=${DDS_MCU}
    -Wl,--gc-sections
    -Wl,-Map=$<TARGET_FILE_DIR:dds_fw>/dds_fw.map,--cref
    -Wl,--print-memory-usage)

set(DDS_HEX "${CMAKE_CURRENT_BINARY_DIR}/dds_fw.hex")
set(DDS_LSS "${CMAKE_CURRENT_BINARY_DIR}/dds_fw.lss")

add_custom_command(TARGET dds_fw POST_BUILD
    COMMAND "${AVR_OBJCOPY}" -O ihex -R .eeprom -R .fuse -R .lock -R .signature
            "$<TARGET_FILE:dds_fw>" "${DDS_HEX}"
    COMMAND "${CMAKE_COMMAND}" "-DCMD=${AVR_OBJDUMP}" "-DARGS=-h|-S|$<TARGET_FILE:dds_fw>"
            "-DOUT=${DDS_LSS}" -P "${CMAKE_CURRENT_SOURCE_DIR}/cmake/run_to_file.cmake"
    COMMAND "${AVR_SIZE}" "$<TARGET_FILE:dds_fw>"
    BYPRODUCTS "${DDS_HEX}" "${DDS_LSS}"
    COMMENT "HEX, листинг и размер прошивки"
    VERBATIM)

# ---------------------------------------------------------------------------
# Программирование: avrdude 7.1+ с профилем 1887ВЕ7Т
# ---------------------------------------------------------------------------
include(cmake/avrdude.cmake)
