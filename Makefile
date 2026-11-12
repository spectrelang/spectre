.PHONY: all cprev clean test_fmt test_critic checkmem checkmem-strict bench_json

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

test_samples:
	@bash ./fw/test.sh

test_fmt:
	@bash ./fw/test_fmt.sh

test_critic:
	@bash ./fw/test_critic.sh

checkmem:
	@bash ./fw/checkmem.sh

checkmem-strict:
	@bash ./fw/checkmem.sh --strict

bench_json:
	@bash ./fw/bench_json.sh

test_surface:
	@bash ./fw/test_alt.sh

test_all:
	@sh ./fw/test.sh -bs
	@bash ./fw/test_alt.sh -bs

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

setup:
	@bash ./fw/setup_devenv.sh
