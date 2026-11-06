# Toolchain-файл CMake для avr-gcc (Windows / Linux / macOS).
#
# avr-gcc ищется в PATH или в каталоге из переменной окружения AVR_TOOLCHAIN_DIR
# (корень тулчейна, в котором лежит bin/avr-gcc[.exe]).

set(CMAKE_SYSTEM_NAME Generic)
set(CMAKE_SYSTEM_PROCESSOR avr)

find_program(AVR_GCC avr-gcc
    HINTS "$ENV{AVR_TOOLCHAIN_DIR}" "$ENV{AVR_TOOLCHAIN_DIR}/bin"
    REQUIRED)
get_filename_component(AVR_BIN_DIR "${AVR_GCC}" DIRECTORY)

set(CMAKE_C_COMPILER   "${AVR_GCC}")
set(CMAKE_ASM_COMPILER "${AVR_GCC}")

find_program(AVR_OBJCOPY avr-objcopy HINTS "${AVR_BIN_DIR}" REQUIRED)
find_program(AVR_OBJDUMP avr-objdump HINTS "${AVR_BIN_DIR}" REQUIRED)
find_program(AVR_SIZE    avr-size    HINTS "${AVR_BIN_DIR}" REQUIRED)

# Проверка компилятора без линковки: для голого железа нет готового main().
set(CMAKE_TRY_COMPILE_TARGET_TYPE STATIC_LIBRARY)

set(CMAKE_FIND_ROOT_PATH_MODE_PROGRAM NEVER)
set(CMAKE_FIND_ROOT_PATH_MODE_LIBRARY ONLY)
set(CMAKE_FIND_ROOT_PATH_MODE_INCLUDE ONLY)

set(CMAKE_EXECUTABLE_SUFFIX_C ".elf")

# Оптимизация: Debug — -Og (удобно шагать в отладчике), Release — -Os.
# _delay_ms()/_delay_us() требуют включённой оптимизации, поэтому не -O0.
set(CMAKE_C_FLAGS_DEBUG_INIT   "-Og -g3")
set(CMAKE_C_FLAGS_RELEASE_INIT "-Os -g -DNDEBUG")
