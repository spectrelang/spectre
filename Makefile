.PHONY: all cprev clean

all:
ifeq ($(OS),Windows_NT)
	@echo "Windows not yet supported"
else
	@sh ./build.sh
endif

cprev:
	@git clean -fdXn

clean:
	@git clean -fdX

bt:
	@sh ./build.sh
	@sh ./test.sh -bs

btfl:
	@sh ./build.sh
	@sh ./test.sh
