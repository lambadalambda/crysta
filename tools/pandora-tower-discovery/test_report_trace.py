#!/usr/bin/env python3
"""Unit tests for the trace reporter. No ROM or session needed."""

from __future__ import annotations

import io
import json
import unittest

import report_trace

RECORD = {
    "kind": "trace",
    "target": "80A395",
    "stop": "TargetReached",
    "instructions": 15030,
    "digest": "deadbeef",
    "elapsed_frames": 1,
    "registers": {
        "address": "80A395",
        "a": 0x0122,
        "x": 0x0122,
        "y": 0x1040,
        "stack": "01F3",
        "direct_page": "0000",
        "status": 0x04,
    },
    "stack_bytes": [0x84, 0xE7, 0xD2, 0x89, 0x5C, 0xC7, 0x80, 0x81],
    "path": ["80A395", "80838D", "80838C"],
    "state": {"map": 65, "position": [120, 192], "flags": [0x20, 0x244]},
}


class TestRender(unittest.TestCase):
    def test_reports_the_stop_reason(self):
        self.assertIn("stop=TargetReached", report_trace.render(RECORD))

    def test_registers_are_hex_and_padded(self):
        text = report_trace.render(RECORD)
        self.assertIn("a=0x0122", text)
        self.assertIn("y=0x1040", text)

    def test_stack_bytes_expose_the_return_address(self):
        # E7 D2 89 is the return into $89D2E7, which identifies the caller.
        self.assertIn("84 E7 D2 89", report_trace.render(RECORD))

    def test_flags_are_rendered_in_hex(self):
        self.assertIn("'0x244'", report_trace.render(RECORD))


class TestLastTrace(unittest.TestCase):
    def test_reads_the_last_trace_record(self):
        lines = [
            json.dumps({"kind": "checkpoint", "label": "boot"}),
            json.dumps(dict(RECORD, target="888888")),
            json.dumps(RECORD),
        ]
        self.assertEqual(
            report_trace.last_trace(io.StringIO("\n".join(lines)))["target"], "80A395"
        )

    def test_ignores_non_trace_records(self):
        stream = io.StringIO(json.dumps({"kind": "profile"}) + "\n")
        self.assertIsNone(report_trace.last_trace(stream))

    def test_no_trace_record_is_an_error_exit(self):
        self.assertEqual(report_trace.report(io.StringIO("")), 1)


if __name__ == "__main__":
    unittest.main()
