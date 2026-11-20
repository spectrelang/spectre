.PHONY: all cprev clean test_fmt test_critic checkmem checkmem-strict bench_json

BLOOD ?= blood

all:
ifeq ($(OS),Windows_NT)
	@echo "Windows not yet supported"
else
	@$(BLOOD) ./fw/build.bx
endif

cprev:
	@git clean -fdXn

clean:
	@git clean -fdX

ba:
	@$(BLOOD) ./fw/bootstrap_all.bx

test_samples:
	@$(BLOOD) ./fw/test.bx

test_fmt:
	@$(BLOOD) ./fw/test_fmt.bx

test_critic:
	@$(BLOOD) ./fw/test_critic.bx

checkmem:
	@$(BLOOD) ./fw/checkmem.bx

checkmem-strict:
	@$(BLOOD) ./fw/checkmem.bx --strict

bench_json:
	@$(BLOOD) ./fw/bench_json.bx

test_surface:
	@$(BLOOD) ./fw/test_alt.bx

test_all:
	@$(BLOOD) ./fw/test.bx -bs
	@$(BLOOD) ./fw/test_alt.bx -bs

bt:
	@$(BLOOD) ./fw/build.bx
	@$(BLOOD) ./fw/test.bx -bs

bta:
	@$(BLOOD) ./fw/test_alt.bx -bs

btfl:
	@$(BLOOD) ./fw/build.bx
	@$(BLOOD) ./fw/test.bx

bs:
	@$(BLOOD) ./fw/build.bx -bs

setup:
	@$(BLOOD) ./fw/setup_devenv.bx
