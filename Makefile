.PHONY: all

all:
ifeq ($(OS),Windows_NT)
	@echo "Windows not yet supported"
else
	@sh ./build.sh
endif
