#!/usr/bin/env python3
"""Mocked recorder wiring controls; never evidence of an emulator execution."""
from contextlib import ExitStack
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import MagicMock, patch

import bridge
import capture
import check
import library_bridge


class RecorderGates(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.base = Path(self.temp.name).resolve()
        self.repo = check.REPO.resolve()
        self.out = self.base / 'run'
        self.rom, self.save = self.base / 'rom', self.base / 'save'
        self.rom.write_bytes(b'rom'); self.save.write_bytes(bytes(8192))
        self.descriptor = dict(source_hashes=check.source_hashes(check.REPO),
                               additional_source_hashes=bridge.additional_hashes(check.REPO))
        self.path = self.base / 'descriptor.json'
        self.path.write_text(json.dumps(self.descriptor))
        exported = self.base / 'export'; exported.mkdir()
        (exported / 'capture.json').write_text('{"synthetic":true}')
        self.process = MagicMock(pid=123, returncode=0)
        self.process.__enter__.return_value = self.process
        self.process.communicate.return_value = (f'Local map viewer: {exported}/index.html\n'.encode(), b'')
        stack = ExitStack(); self.addCleanup(stack.close)
        self.verify = stack.enter_context(patch.object(bridge, 'verify_current', return_value=self.descriptor))
        stack.enter_context(patch.object(capture, 'ROM', check.sha(self.rom.read_bytes())))
        stack.enter_context(patch.object(capture, 'SRAM', check.sha(self.save.read_bytes())))
        self.build = stack.enter_context(patch.object(capture.subprocess, 'run', side_effect=self.fake_build))
        self.spawn = stack.enter_context(patch.object(capture.subprocess, 'Popen', return_value=self.process))
        stack.enter_context(patch.object(capture.subprocess, 'check_output', return_value=b'tool\n'))
        stack.enter_context(patch('builtins.print'))

    def fake_build(self, command, *, cwd, env, stdout, stderr, check):
        self.assertEqual(command, ['cargo', 'build', '--locked', '-p', 'map-inspector'])
        self.assertEqual(cwd, self.repo)
        target = Path(env['CARGO_TARGET_DIR'])
        self.assertEqual(target, self.out / 'target')
        self.assertFalse(target.exists())
        (target / 'debug').mkdir(parents=True)
        (target / 'debug/map-inspector').write_bytes(b'synthetic executable')
        stdout.write(b'synthetic build log')

    def record(self):
        capture.run(check.REPO, self.out, self.rom, self.save, self.path, self.base / 'fixed')

    def test_fresh_target_one_process_and_source_checks(self):
        self.record()
        self.build.assert_called_once()
        self.spawn.assert_called_once()
        self.assertEqual(self.spawn.call_args.args[0],
                         [str(self.out / 'map-inspector'), 'capture', str(self.rom), str(self.save)])
        self.assertEqual(self.verify.call_count, 2)
        self.assertEqual(self.verify.call_args_list[0], self.verify.call_args_list[1])
        producer = check.load(self.out / 'producer.json')
        self.assertEqual(producer['schema_version'], 2)
        self.assertEqual(producer['descriptor_sha256'], check.sha(self.path.read_bytes()))
        self.assertEqual(producer['additional_source_hashes'], self.descriptor['additional_source_hashes'])
        self.assertTrue(producer['fresh_target'])
        self.assertEqual(producer['process']['pid'], 123)
        self.assertLess(producer['process']['started_ns'], producer['process']['finished_ns'])
        self.assertTrue(producer['process']['run_id'])

    def test_existing_output_is_not_reused(self):
        self.out.mkdir()
        with self.assertRaises(FileExistsError):
            self.record()
        self.build.assert_not_called()
        self.spawn.assert_not_called()

    def test_source_change_after_capture_is_rejected(self):
        self.verify.side_effect = [self.descriptor, ValueError('additional source mismatch')]
        with self.assertRaisesRegex(ValueError, 'source mismatch'):
            self.record()
        self.assertFalse((self.out / 'producer.json').exists())
        self.spawn.assert_called_once()

    def test_descriptor_bytes_changed_after_capture_are_rejected(self):
        def changed(*args, **kwargs):
            self.path.write_bytes(self.path.read_bytes() + b'\n')
            return self.process.communicate.return_value
        self.process.communicate.side_effect = changed
        with self.assertRaisesRegex(ValueError, 'descriptor changed'):
            self.record()
        self.assertFalse((self.out / 'producer.json').exists())

    def test_library_mode_rechecks_and_records_exact_schema_three_groups(self):
        descriptor = check.load(check.HERE / 'library-producer.json')
        predecessor = check.load(check.HERE / 'current-producer.json')
        self.path.write_text(json.dumps(descriptor))
        with patch.object(library_bridge, 'verify_library_descriptor', return_value=descriptor) as verify, \
                patch.object(library_bridge, 'frozen_predecessor',
                             return_value=(None, predecessor, None)):
            capture.run(self.repo, self.out, self.rom, self.save,
                        fixed_source_repo=self.base / 'fixed',
                        library_descriptor=self.path,
                        predecessor_source_repo=self.base / 'predecessor')
        self.assertEqual(verify.call_count, 2)
        producer = check.load(self.out / 'producer.json')
        self.assertEqual(producer['schema_version'], 3)
        self.assertEqual(producer['kind'], library_bridge.KIND)
        self.assertEqual(producer['source_hashes'],
                         library_bridge.current_sources(descriptor, predecessor))
        for field in ('replaced_source_hashes', *library_bridge.GROUPS):
            self.assertEqual(producer[field], descriptor[field])

    def test_library_mode_requires_all_explicit_source_repositories(self):
        with self.assertRaisesRegex(ValueError, 'fixed source'):
            capture.run(self.repo, self.out, self.rom, self.save,
                        library_descriptor=self.path,
                        predecessor_source_repo=self.base / 'predecessor')
        with self.assertRaisesRegex(ValueError, 'predecessor source'):
            capture.run(self.repo, self.out, self.rom, self.save,
                        library_descriptor=self.path,
                        fixed_source_repo=self.base / 'fixed')
        self.assertFalse(self.out.exists())


if __name__ == '__main__':
    unittest.main()
