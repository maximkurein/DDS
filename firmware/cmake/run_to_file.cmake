# Запуск программы с выводом stdout в файл (переносимо, без перенаправления оболочки).
#   cmake -DCMD=<exe> -DARGS=<a|b|c> -DOUT=<file> -P run_to_file.cmake
string(REPLACE "|" ";" _args "${ARGS}")
execute_process(COMMAND "${CMD}" ${_args}
    OUTPUT_FILE "${OUT}"
    RESULT_VARIABLE _rc)
if(NOT _rc EQUAL 0)
    message(FATAL_ERROR "${CMD} failed: ${_rc}")
endif()
