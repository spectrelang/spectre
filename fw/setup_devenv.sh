#!/usr/bin/env bash

cc -O2 -c "./spectrelib/csources/panic_handler.c" -o "./spectrelib/csources/panic_handler.o"
cc -O2 -c "./spectrelib/csources/yyjson_shim.c" -o "./spectrelib/csources/yyjson_shim.o"
