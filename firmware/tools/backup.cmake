# Резервная копия содержимого МК перед стиранием или сменой фьюзов.
#   cmake -DAVRDUDE_CMD="avrdude|..." -DOUTDIR=<dir> -P backup.cmake
# ТО 1887ВЕ7Т и сводка проекта: перед изменениями сохранить фьюзы, lock,
# калибровку, программу и EEPROM.

string(REPLACE "|" ";" _ad "${AVRDUDE_CMD}")
string(TIMESTAMP _ts "%Y%m%d_%H%M%S")
set(_dir "${OUTDIR}/${_ts}")
file(MAKE_DIRECTORY "${_dir}")

execute_process(COMMAND ${_ad}
        -U "flash:r:${_dir}/flash.hex:i"
        -U "eeprom:r:${_dir}/eeprom.hex:i"
        -U "lfuse:r:${_dir}/lfuse.txt:h"
        -U "hfuse:r:${_dir}/hfuse.txt:h"
        -U "efuse:r:${_dir}/efuse.txt:h"
        -U "lock:r:${_dir}/lock.txt:h"
        -U "calibration:r:${_dir}/calibration.txt:h"
    RESULT_VARIABLE _rc)
if(NOT _rc EQUAL 0)
    message(FATAL_ERROR "Резервное копирование не удалось (avrdude: ${_rc})")
endif()
message(STATUS "Сохранено в ${_dir}")
