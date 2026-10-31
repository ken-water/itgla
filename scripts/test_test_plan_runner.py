"""Self-tests for orchestration outcomes, timeout handling and profile boundaries."""
import importlib.util
import os
from pathlib import Path
import sys
import tempfile
import unittest

sys.dont_write_bytecode = True
spec = importlib.util.spec_from_file_location("runner", Path(__file__).with_name("run-test-plan.py"))
runner = importlib.util.module_from_spec(spec)
spec.loader.exec_module(runner)


class RunnerTests(unittest.TestCase):
    def run_python(self, source, timeout=5):
        with tempfile.TemporaryDirectory() as directory:
            gate = dict(id="fixture", command=[sys.executable, "-c", source],
                        cwd=directory, timeout=timeout)
            result = runner.run_gate(gate, Path(directory), dict(os.environ))
            return result, (Path(directory) / "fixture.log").read_text(encoding="utf-8")

    def test_pass_and_failure_preserve_output_and_code(self):
        result, log = self.run_python("print('fixture evidence')")
        self.assertEqual(result["status"], "PASS")
        self.assertIn("fixture evidence", log)
        result, _ = self.run_python("raise SystemExit(7)")
        self.assertEqual((result["status"], result["exit_code"]), ("FAIL", 7))

    def test_timeout_is_not_pass(self):
        result, _ = self.run_python("import time; time.sleep(30)", timeout=0.1)
        self.assertEqual(result["status"], "TIMEOUT")
        self.assertLess(result["seconds"], 10)

    def test_missing_tool_is_blocked(self):
        with tempfile.TemporaryDirectory() as directory:
            gate = dict(id="absent", command=[str(Path(directory) / "no-tool")], cwd=directory, timeout=1)
            result = runner.run_gate(gate, Path(directory), dict(os.environ))
            self.assertEqual(result["status"], "BLOCKED")

    def test_missing_workdir_is_blocked(self):
        with tempfile.TemporaryDirectory() as directory:
            gate = dict(id="absent", command=[sys.executable], cwd=str(Path(directory) / "absent"), timeout=1)
            self.assertEqual(runner.run_gate(gate, Path(directory), dict(os.environ))["status"], "BLOCKED")

    def test_aggregate_never_turns_missing_checks_into_pass(self):
        for statuses, expected in [(["PASS"], 0), (["PASS", "BLOCKED"], 2),
                                   (["NOT_RUN"], 2), (["TIMEOUT"], 1),
                                   (["FAIL", "BLOCKED"], 1), (["INTERRUPTED"], 1)]:
            self.assertEqual(runner.exit_code([dict(status=s) for s in statuses]), expected)

    def test_profiles_are_locked_and_quick_is_focused(self):
        quick = runner.commands("quick", "1.92.0")
        full = runner.commands("full", "1.92.0")
        release = runner.commands("release", "1.92.0")
        self.assertIn("asset_context::tests::", next(g["command"] for g in quick if g["id"] == "rust-tests"))
        self.assertNotIn("asset_context::tests::", next(g["command"] for g in full if g["id"] == "rust-tests"))
        self.assertIn("release-build", [g["id"] for g in release])
        self.assertNotIn("release-build", [g["id"] for g in full])
        for profile in [quick, full, release]:
            for gate in profile:
                if gate["id"] in ["rust-tests", "clippy", "release-build", "coverage"]:
                    self.assertIn("--locked", gate["command"])
                    self.assertEqual(gate["command"][gate["command"].index("-j") + 1], "2")

    def test_case_catalog_is_actionable_and_unique(self):
        import json
        cases = json.loads((runner.ROOT / "docs/testing/phase-one-cases.json").read_text(encoding="utf-8"))
        self.assertEqual(len({c["id"] for c in cases}), len(cases))
        for case in cases:
            self.assertIn(case["priority"], ["P0", "P1", "P2"])
            self.assertIn(case["coverage"], ["AUTO+MANUAL", "MANUAL"])
            for field in ["title", "steps", "expected", "automation"]:
                self.assertTrue(case[field], (case["id"], field))


if __name__ == "__main__":
    unittest.main()
