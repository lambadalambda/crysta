"""Publication tripwires only; canonical boundary validation lives in Rust."""
import copy
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("cadence_run", Path(__file__).with_name("run.py"))
runner = importlib.util.module_from_spec(spec)
spec.loader.exec_module(runner)


class PublicationTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.pending = Path(self.directory.name) / "proof.pending.json"
        self.output = Path(self.directory.name) / "proof.json"
        self.provenance = {"generator_revision": "a" * 40, "source_archive_sha256": "b" * 64}
        self.proof = {
            "schema": 1, "provenance": self.provenance,
            "offline": [None] * 11591, "projected": [None] * 11410,
            "qualification": {"original_actions": 11590, "projected_actions": 11409,
                              "omitted_indices": list(range(181)),
                              "retained_visible_arrival_updates": {"CEntry": 17, "BoxEntry": 1},
                              "raw_core_results": "all-success-exact-host-match"},
        }

    def write_pending(self, proof=None):
        self.pending.write_text(json.dumps(self.proof if proof is None else proof))

    def test_success_publishes_exact_bytes_without_replacement(self):
        self.write_pending()
        before = self.pending.read_bytes()
        runner.publish(self.pending, self.output, self.provenance)
        self.assertEqual(self.output.read_bytes(), before)
        self.assertFalse(self.pending.exists())
        self.write_pending()
        with self.assertRaises(ValueError):
            runner.publish(self.pending, self.output, self.provenance)
        self.assertEqual(self.output.read_bytes(), before)

    def test_partial_or_different_evidence_never_publishes(self):
        cases = []
        for key in ("schema", "provenance", "offline", "projected", "qualification"):
            bad = copy.deepcopy(self.proof)
            del bad[key]
            cases.append(bad)
        for key in self.proof["qualification"]:
            bad = copy.deepcopy(self.proof)
            del bad["qualification"][key]
            cases.append(bad)
        bad = copy.deepcopy(self.proof)
        bad["provenance"]["generator_revision"] = "c" * 40
        cases.append(bad)
        for bad in cases:
            self.write_pending(bad)
            with self.assertRaises(ValueError):
                runner.publish(self.pending, self.output, self.provenance)
            self.assertFalse(self.output.exists())
            self.assertTrue(self.pending.exists())

    def test_truncated_json_and_failed_rename_leave_no_proof(self):
        self.pending.write_text('{"schema":1')
        with self.assertRaises(ValueError):
            runner.publish(self.pending, self.output, self.provenance)
        self.assertFalse(self.output.exists())
        self.write_pending()
        with patch.object(Path, "rename", side_effect=OSError("disk failure")):
            with self.assertRaises(OSError):
                runner.publish(self.pending, self.output, self.provenance)
        self.assertFalse(self.output.exists())

    def test_digest_failure_prevents_publication(self):
        self.write_pending()
        with patch.object(runner, "digest", side_effect=OSError("read failure")):
            with self.assertRaises(OSError):
                runner.publish(self.pending, self.output, self.provenance)
        self.assertFalse(self.output.exists())
    def test_checksum_failure_prevents_publication(self):
        self.write_pending()
        with patch.object(Path, "write_text", side_effect=OSError("checksum failure")):
            with self.assertRaises(OSError):
                runner.publish(self.pending, self.output, self.provenance)
        self.assertFalse(self.output.exists())

    def test_final_identity_or_reporting_failure_prevents_publication(self):
        self.write_pending()
        def failure():
            raise ValueError("identity changed or reporting failed")
        with self.assertRaises(ValueError):
            runner.publish(self.pending, self.output, self.provenance, failure)
        self.assertFalse(self.output.exists())


if __name__ == "__main__":
    unittest.main()
