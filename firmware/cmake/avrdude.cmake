# Цели программирования 1887ВЕ7Т через avrdude 7.1+ и Atmel-ICE.
#
# Все параметры — в кэше CMake (меняются в CMakePresets.json, CMakeUserPresets.json
# или командой cmake -D...):
#   AVRDUDE_PROGRAMMER  atmelice      — JTAG через разъём X3 (основной путь)
#                       atmelice_isp  — ISP через площадки X1 (резерв, восстановление JTAGEN)
#   AVRDUDE_BITCLOCK    период такта JTAG/ISP в микросекундах (-B):
#                       8 → 125 кГц — безопасно для нового МК (заводской RC 1 МГц);
#                       1 → 1 МГц   — после записи фьюзов на RC 8 МГц.
#                       Правило: частота TCK/SCK < F_CPU / 4.

find_program(AVRDUDE avrdude HINTS "$ENV{AVRDUDE_DIR}")
if(NOT AVRDUDE)
    message(WARNING "avrdude не найден: цели программирования недоступны (задайте PATH или AVRDUDE_DIR)")
    return()
endif()

set(AVRDUDE_PROGRAMMER "atmelice" CACHE STRING "atmelice (JTAG, X3) | atmelice_isp (ISP, X1)")
set_property(CACHE AVRDUDE_PROGRAMMER PROPERTY STRINGS atmelice atmelice_isp)
set(AVRDUDE_PORT     "usb"      CACHE STRING "Порт программатора (-P)")
set(AVRDUDE_BITCLOCK "8"        CACHE STRING "Период такта JTAG/ISP, мкс (-B)")
set(AVRDUDE_PART     "1887ve7t" CACHE STRING "Профиль avrdude (-p)")
set(AVRDUDE_EXTRA    ""         CACHE STRING "Дополнительные ключи avrdude")

set(AVRDUDE_CONF "${CMAKE_CURRENT_SOURCE_DIR}/tools/avrdude/1887ve7t.conf")
set(DDS_BACKUP_DIR "${CMAKE_CURRENT_SOURCE_DIR}/backup")

set(_ad ${AVRDUDE}
    -C "+${AVRDUDE_CONF}"
    -p ${AVRDUDE_PART}
    -c ${AVRDUDE_PROGRAMMER}
    -P ${AVRDUDE_PORT}
    -B ${AVRDUDE_BITCLOCK}
    ${AVRDUDE_EXTRA})

# Прошивка без стирания (-D). Память программ 1887ВЕ7Т допускает перезапись
# без предварительного стирания (ТО КФДЛ.431295.044ТО, с. 29), а РЭ
# КФДЛ.301411.243РЭ (с. 31–33) советует не стирать автоматически перед записью.
# Проверка чтением после записи у avrdude включена по умолчанию.
add_custom_target(flash
    COMMAND ${_ad} -D -U "flash:w:${DDS_HEX}:i"
    DEPENDS dds_fw
    COMMENT "Запись ${DDS_HEX} (${AVRDUDE_PROGRAMMER}, без стирания)"
    USES_TERMINAL VERBATIM)

add_custom_target(flash_clean
    COMMAND ${_ad} -e -U "flash:w:${DDS_HEX}:i"
    DEPENDS dds_fw
    COMMENT "Стирание кристалла и запись (EEPROM сохраняется при EESAVE=0)"
    USES_TERMINAL VERBATIM)

add_custom_target(verify
    COMMAND ${_ad} -U "flash:v:${DDS_HEX}:i"
    DEPENDS dds_fw
    COMMENT "Сверка памяти программ с ${DDS_HEX}"
    USES_TERMINAL VERBATIM)

add_custom_target(probe
    COMMAND ${_ad} -v
    COMMENT "Связь с МК: напряжение цели, сигнатура 1E 97 02"
    USES_TERMINAL VERBATIM)

add_custom_target(erase
    COMMAND ${_ad} -e
    COMMENT "Стирание кристалла"
    USES_TERMINAL VERBATIM)

add_custom_target(fuses_read
    COMMAND ${_ad} -U lfuse:r:-:h -U hfuse:r:-:h -U efuse:r:-:h -U lock:r:-:h
    COMMENT "Чтение фьюзов и lock"
    USES_TERMINAL VERBATIM)

add_custom_target(fuses_write
    COMMAND ${_ad} -U "lfuse:w:${DDS_FUSE_LOW}:m" -U "hfuse:w:${DDS_FUSE_HIGH}:m" -U "efuse:w:${DDS_FUSE_EXT}:m"
    COMMENT "Запись фьюзов L=${DDS_FUSE_LOW} H=${DDS_FUSE_HIGH} E=${DDS_FUSE_EXT}"
    USES_TERMINAL VERBATIM)

string(REPLACE ";" "|" _ad_pipe "${_ad}")

add_custom_target(osccal
    COMMAND "${CMAKE_COMMAND}" "-DAVRDUDE_CMD=${_ad_pipe}" "-DWORKDIR=${CMAKE_CURRENT_BINARY_DIR}"
            -P "${CMAKE_CURRENT_SOURCE_DIR}/tools/osccal.cmake"
    COMMENT "Калибровка RC 8 МГц: чтение калибровочных байтов и запись в EEPROM[0..1]"
    USES_TERMINAL VERBATIM)

add_custom_target(backup
    COMMAND "${CMAKE_COMMAND}" "-DAVRDUDE_CMD=${_ad_pipe}" "-DOUTDIR=${DDS_BACKUP_DIR}"
            -P "${CMAKE_CURRENT_SOURCE_DIR}/tools/backup.cmake"
    COMMENT "Резервная копия flash, EEPROM, фьюзов, lock и калибровки в ${DDS_BACKUP_DIR}"
    USES_TERMINAL VERBATIM)
