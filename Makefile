.PHONY: all cprev clean

all:
ifeq ($(OS),Windows_NT)
	@echo "Windows not yet supported"
else
	@sh ./fw/build.sh
endif

cprev:
	@git clean -fdXn

clean:
	@git clean -fdX

ba:
	@bash ./fw/bootstrap_all.sh

bt:
	@sh ./fw/build.sh
	@sh ./fw/test.sh -bs

bta:
	@bash ./fw/test_alt.sh -bs

btfl:
	@sh ./fw/build.sh
	@sh ./fw/test.sh

bs:
	@sh ./fw/build.sh -bs
