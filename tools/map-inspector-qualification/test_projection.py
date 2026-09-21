#!/usr/bin/env python3
"""The projected build-input pins must stay strict in both directions."""
import hashlib
from pathlib import Path
import unittest

import projection

REPO = Path(__file__).resolve().parent.parent.parent

SAMPLE_LOCK = '''
[[package]]
name = "map-inspector"
version = "0.1.0"
dependencies = [
 "oracle",
]

[[package]]
name = "oracle"
version = "0.1.0"
source = "registry+x"
checksum = "abc"
dependencies = [
 "cc",
]

[[package]]
name = "cc"
version = "1.1.0"
source = "registry+x"
checksum = "def"

[[package]]
name = "unrelated"
version = "9.9.9"
source = "registry+x"
checksum = "999"
'''

SAMPLE_MANIFEST = '''[workspace]
resolver = "2"
members = ["crates/rom", "crates/oracle"]

[workspace.package]
version = "0.1.0"

[workspace.lints.clippy]
pedantic = "warn"
'''


class LockfileProjection(unittest.TestCase):
    def test_reaches_the_whole_closure_and_nothing_else(self):
        projected = projection.lockfile_subtree(SAMPLE_LOCK)
        for name in ('map-inspector', 'oracle', 'cc'):
            self.assertIn(name, projected)
        self.assertNotIn('unrelated', projected)

    def test_ignores_packages_outside_the_closure(self):
        added = SAMPLE_LOCK + (
            '\n[[package]]\nname = "newcomer"\nversion = "0.1.0"\n'
            'dependencies = [\n "cc",\n]\n'
        )
        self.assertEqual(projection.lockfile_subtree(SAMPLE_LOCK),
                         projection.lockfile_subtree(added))

    def test_catches_every_change_inside_the_closure(self):
        baseline = projection.lockfile_subtree(SAMPLE_LOCK)
        for old, new in (
            ('version = "1.1.0"', 'version = "1.2.0"'),
            ('checksum = "def"', 'checksum = "deadbeef"'),
            ('dependencies = [\n "cc",\n]', 'dependencies = [\n]'),
        ):
            mutated = SAMPLE_LOCK.replace(old, new, 1)
            self.assertNotEqual(mutated, SAMPLE_LOCK, old)
            self.assertNotEqual(projection.lockfile_subtree(mutated), baseline, old)

    def test_a_fork_sharing_name_and_version_is_not_merged(self):
        # Keying on (name, version) alone drops the fork's edges and checksums.
        lock = '''
[[package]]
name = "map-inspector"
version = "0.1.0"
dependencies = [
 "foo 1.0.0 (registry+x)",
 "bar",
]

[[package]]
name = "bar"
version = "1.0.0"
source = "registry+x"
checksum = "bar-sum"
dependencies = [
 "foo 1.0.0 (git+fork)",
]

[[package]]
name = "foo"
version = "1.0.0"
source = "registry+x"
checksum = "registry-sum"

[[package]]
name = "foo"
version = "1.0.0"
source = "git+fork"
checksum = "fork-sum"
dependencies = [
 "hidden",
]

[[package]]
name = "hidden"
version = "6.6.6"
source = "registry+x"
checksum = "hidden-sum"
'''
        projected = projection.lockfile_subtree(lock)
        for sum_name in ('registry-sum', 'fork-sum', 'hidden-sum'):
            self.assertIn(sum_name, projected)
            mutated = lock.replace(sum_name, 'tampered', 1)
            self.assertNotEqual(projection.lockfile_subtree(mutated), projected, sum_name)


class ManifestProjection(unittest.TestCase):
    def test_elides_only_the_member_list(self):
        baseline = projection.manifest_without_members(SAMPLE_MANIFEST)
        added = SAMPLE_MANIFEST.replace(
            'members = ["crates/rom", "crates/oracle"]',
            'members = ["crates/rom", "crates/oracle", "crates/newcomer"]')
        self.assertEqual(projection.manifest_without_members(added), baseline)
        multiline = SAMPLE_MANIFEST.replace(
            'members = ["crates/rom", "crates/oracle"]',
            'members = [\n  "crates/rom",\n  "crates/oracle",\n]')
        self.assertEqual(projection.manifest_without_members(multiline), baseline)

    def test_catches_everything_else(self):
        baseline = projection.manifest_without_members(SAMPLE_MANIFEST)
        for old, new in (
            ('resolver = "2"', 'resolver = "1"'),
            ('version = "0.1.0"', 'version = "0.2.0"'),
            ('pedantic = "warn"', 'pedantic = "allow"'),
        ):
            mutated = SAMPLE_MANIFEST.replace(old, new, 1)
            self.assertNotEqual(mutated, SAMPLE_MANIFEST, old)
            self.assertNotEqual(projection.manifest_without_members(mutated), baseline, old)
        profiled = SAMPLE_MANIFEST + '\n[profile.release]\nlto = true\n'
        self.assertNotEqual(projection.manifest_without_members(profiled), baseline)

    def test_the_elision_cannot_run_past_its_own_line(self):
        baseline = projection.manifest_without_members(SAMPLE_MANIFEST)
        smuggled = SAMPLE_MANIFEST.replace(
            'members = ["crates/rom", "crates/oracle"]',
            'members = ["crates/rom", "crates/oracle"] #\n'
            'dependencies = { serde = "1" }\n[patch.crates-io]')
        projected = projection.manifest_without_members(smuggled)
        self.assertNotEqual(projected, baseline)
        self.assertIn('patch.crates-io', projected)

    def test_only_the_workspace_tables_members_key_is_elided(self):
        elsewhere = SAMPLE_MANIFEST + '\n[workspace.metadata.x]\nmembers_are_cool = "yes"\n'
        self.assertIn('members_are_cool', projection.manifest_without_members(elsewhere))
        defaults = SAMPLE_MANIFEST.replace(
            'members = ["crates/rom", "crates/oracle"]',
            'members = ["crates/rom"]\ndefault-members = ["crates/rom"]')
        self.assertIn('default-members', projection.manifest_without_members(defaults))


class EffectiveSha(unittest.TestCase):
    def test_the_live_tree_projects_onto_the_frozen_fixtures(self):
        # The binding the descriptor relies on: live file -> fixture -> pin.
        for name in projection.PROJECTED:
            fixture = (projection.PINNED / name).read_bytes()
            self.assertEqual(projection.effective_sha(REPO, name),
                             hashlib.sha256(fixture).hexdigest(), name)

    def test_a_real_change_still_reports_the_live_hash(self):
        # effective_sha must not launder a genuine change into the pin.
        scratch = Path(__file__).resolve().parent / 'local-projection-scratch'
        scratch.mkdir(exist_ok=True)
        try:
            for name, mutation in (
                ('Cargo.lock', lambda t: t.replace('name = "rom"', 'name = "rom-evil"', 1)),
                ('Cargo.toml', lambda t: t.replace('resolver = "2"', 'resolver = "1"', 1)),
            ):
                text = (projection.PINNED / name).read_text()
                mutated = mutation(text)
                self.assertNotEqual(mutated, text, name)
                (scratch / name).write_text(mutated)
                self.assertEqual(
                    projection.effective_sha(scratch, name),
                    hashlib.sha256(mutated.encode()).hexdigest(),
                    f'{name} must report its own hash when genuinely changed')
                (scratch / name).unlink()
        finally:
            if scratch.exists() and not any(scratch.iterdir()):
                scratch.rmdir()


if __name__ == '__main__':
    unittest.main()
