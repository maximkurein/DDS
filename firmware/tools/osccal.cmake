# Перенос калибровки RC 8 МГц в EEPROM.
#   cmake -DAVRDUDE_CMD="avrdude|-C|+conf|-p|1887ve7t|..." -DWORKDIR=<dir> -P osccal.cmake
#
# 1887ВЕ7Т (как ATmega128) хранит 4 калибровочных байта: для 1, 2, 4 и 8 МГц.
# При сбросе в OSCCAL загружается байт для 1 МГц. Прошивка берёт байт для
# 8 МГц из EEPROM[0] и проверяет его по дополнению в EEPROM[1].

string(REPLACE "|" ";" _ad "${AVRDUDE_CMD}")
set(_cal "${WORKDIR}/calibration.txt")

execute_process(COMMAND ${_ad} -U "calibration:r:${_cal}:h" RESULT_VARIABLE _rc)
if(NOT _rc EQUAL 0)
    message(FATAL_ERROR "Не удалось прочитать калибровочные байты (avrdude: ${_rc})")
endif()

file(READ "${_cal}" _text)
string(REGEX MATCHALL "0x[0-9a-fA-F]+" _bytes "${_text}")
list(LENGTH _bytes _n)
if(_n LESS 4)
    message(FATAL_ERROR "Ожидалось 4 калибровочных байта, прочитано: '${_text}'")
endif()

list(GET _bytes 3 _b8)
math(EXPR _v   "${_b8}"             OUTPUT_FORMAT HEXADECIMAL)
math(EXPR _inv "(~${_b8}) & 0xFF"   OUTPUT_FORMAT HEXADECIMAL)
message(STATUS "Калибровка 1/2/4/8 МГц: ${_bytes}")
message(STATUS "В EEPROM[0..1] пишется ${_v}, ${_inv} (8 МГц и дополнение)")

execute_process(COMMAND ${_ad} -U "eeprom:w:${_v},${_inv}:m" RESULT_VARIABLE _rc)
if(NOT _rc EQUAL 0)
    message(FATAL_ERROR "Запись EEPROM не удалась (avrdude: ${_rc})")
endif()
message(STATUS "Готово. Прошивка применит калибровку при следующем старте.")
