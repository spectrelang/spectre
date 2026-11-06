# Compiler leak harnesses

Run all harnesses with:

```sh
./fw/checkmem.sh
# or
make checkmem
```

By default the script reports leak findings but only exits non-zero when a
harness fails to build or run. Pass `--strict` to make any leak finding fail
the command.

```sh
./fw/checkmem.sh --strict
```

Each `fw/harness/<module>.sx` file imports one compiler module, exercises it in
a named test block, and calls the shared `lib/check.sx` helper. The helper
reads the `LeakDiagnostics` returned by `@dumpmemleaks` and fails when the
target module leaves new live allocations or records invalid frees.

`lib/check.sx` can take a baseline before a target runs. This keeps setup work
from other modules out of the report: for example, the codegen harness resolves
and checks a fixture first, then only attributes allocations made by codegen to
codegen.

The fixture in `fixtures/smoke.sx` is intentionally small and self-contained so
the frontend/codegen harnesses do not depend on external samples.

Harnesses are compiled with `-test -leak-check`; logs are written below
`${TEST_LOG_DIR:-${RUNNER_TEMP:-${TMPDIR:-/tmp}}}/spectre-checkmem-logs`.
