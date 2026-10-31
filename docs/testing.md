# Testing ITGLA

The current [complete test plan](testing/phase-one-test-plan.md) covers phase-one responsibility context and existing product regressions. The [case catalog](testing/phase-one-cases.json) records executable steps, expected outcomes and partial/manual coverage boundaries.

```text
python scripts/run-test-plan.py --profile quick
python scripts/run-test-plan.py --profile full
python scripts/run-test-plan.py --profile release --list
```

Reports, logs and a blank acceptance checklist are retained in a unique directory under `test-results/`. A successful selected profile does not establish release readiness. See the full plan for prerequisites, exit codes, native/platform checks and the optional website gate.

Prepare a fresh native acceptance directory using an exact candidate binary:

```text
python scripts/start-acceptance.py --binary dist/phase-one-preview/ITGLA.exe
python scripts/start-acceptance.py --binary dist/phase-one-preview/ITGLA.exe --launch
```

Each command creates a separate workspace and records the binary hash. The first command prepares evidence only; the second also opens the application. Choose Open sample data inside the new application if a fictional demo is needed. Neither command opens the normal user database, upgrades an installer or closes another running preview. Specify a Linux/macOS candidate executable path when using those platforms; installation lifecycle tests still require the actual packages.
