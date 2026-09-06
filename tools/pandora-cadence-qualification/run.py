#!/usr/bin/env python3
"""Private, clean-source canonical cadence producer; no server or browser session."""
import argparse
import datetime
import hashlib
import json
import os
from pathlib import Path
import re
import shlex
import subprocess

ROOT = Path(__file__).resolve().parents[2]
PREFIX = "room_preview::cadence_tests::"
GENERATOR = PREFIX + "generate_canonical_cadence_proof"
REJECTION = PREFIX + "raw_rejection_cannot_be_hidden_by_host_noop"


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def publish(pending, output, provenance, before_commit=lambda: None):
    """Prepare all evidence first; final rename is the publication commit point."""
    if output.exists():
        raise ValueError("refusing to replace an existing proof")
    proof = json.loads(pending.read_bytes())
    if proof.get("schema") != 1 or any(
        proof.get("provenance", {}).get(key) != value
        for key, value in provenance.items()
    ):
        raise ValueError("proof provenance differs from wrapper evidence")
    qualification = proof.get("qualification", {})
    if (
        len(proof.get("offline", [])) != 11591
        or len(proof.get("projected", [])) != 11410
        or qualification.get("original_actions") != 11590
        or qualification.get("projected_actions") != 11409
        or len(qualification.get("omitted_indices", [])) != 181
        or qualification.get("retained_visible_arrival_updates") != {"CEntry": 17, "BoxEntry": 1}
        or qualification.get("raw_core_results") != "all-success-exact-host-match"
    ):
        raise ValueError("incomplete qualified proof")
    checksum = digest(pending)
    output.with_suffix(".sha256").write_text(checksum + "  proof.json\n")
    before_commit()
    # No required hashing, logging or reporting after this atomic commit point.
    pending.rename(output)


def run(rom, profile, name):
    if not re.fullmatch(r"[A-Za-z0-9_-]+", name):
        raise ValueError("run name must be a single alphanumeric/dash/underscore component")
    os.chdir(ROOT)
    # Fail before creating output when source is dirty. Ignored private output is
    # intentional; tracked AND untracked source changes are forbidden.
    def identity():
        status = subprocess.check_output(["git", "status", "--porcelain", "--untracked-files=all"])
        if status:
            raise ValueError("clean committed source required:\n" + status.decode())
        return tuple(subprocess.check_output(["git", "rev-parse", ref]).decode().strip()
                     for ref in ("HEAD", "HEAD^{tree}"))

    revision, tree = identity()
    output = ROOT / "local" / "pandora-cadence-qualification" / name / profile
    output.mkdir(parents=True, exist_ok=False)
    env = os.environ.copy()
    with (output / "commands.log").open("w") as log:
        def command(args, filename, binary=False):
            args = list(map(str, args))
            log.write("$ " + shlex.join(args) + "\n")
            log.flush()
            with (output / filename).open("wb") as stream:
                result = subprocess.run(args, stdout=stream,
                                        stderr=log if binary else subprocess.STDOUT, env=env)
            log.write(f"exit={result.returncode}\n")
            log.flush()
            result.check_returncode()
            return output / filename

        log.write(f"cwd={ROOT}\nrevision={revision}\ntree={tree}\nprofile={profile}\n")
        # Include exact build-affecting environment values in private evidence.
        log.write("build_environment=" + json.dumps({k: v for k, v in env.items()
                  if k.startswith(("CARGO", "RUST", "CC", "CXX", "SDK", "MACOSX"))}, sort_keys=True) + "\n")
        archive = command(["git", "archive", "--format=tar", "HEAD"], "source.tar", binary=True)
        rustc = command(["rustc", "-vV"], "rustc-vV.txt")
        command(["cargo", "-V"], "cargo-version.txt")
        args = ["cargo", "test", "--locked", "-p", "map-inspector", "--bin", "map-inspector",
                "--no-run", "--message-format=json"]
        if profile == "release":
            args.append("--release")
        build = command(args, "build.jsonl", binary=True)
        executables = {message["executable"] for line in build.read_text().splitlines()
                       if (message := json.loads(line)).get("reason") == "compiler-artifact"
                       and message.get("executable") and message.get("profile", {}).get("test")}
        if len(executables) != 1:
            raise ValueError("expected exactly one host test executable")
        executable = Path(executables.pop())
        listing = command([executable, "--list"], "tests.txt").read_text().splitlines()
        for test in (GENERATOR, REJECTION, PREFIX + "proof_validator_requires_complete_fresh_replay_at_every_retained_boundary",
                     PREFIX + "hashing_copy_changes_only_global_tick_and_never_the_input"):
            if test + ": test" not in listing:
                raise ValueError("missing test/module hook: " + test)
        provenance = {
            "generator_revision": revision, "source_tree": tree, "build_profile": profile,
            "source_archive_sha256": digest(archive), "rustc_vv_sha256": digest(rustc),
            "cargo_lock_sha256": digest(ROOT / "Cargo.lock"),
            "executable_sha256": digest(executable),
        }
        provenance_path = output / "provenance.json"
        provenance_path.write_text(json.dumps(provenance, indent=2) + "\n")
        pending = output / "proof.pending.json"
        env.update(PANDORA_CADENCE_ROM=str(rom), PANDORA_CADENCE_PENDING=str(pending),
                   PANDORA_CADENCE_PROVENANCE=str(provenance_path))
        log.write("test_environment=" + json.dumps({k: v for k, v in env.items()
                  if k.startswith("PANDORA_CADENCE_")}, sort_keys=True) + "\n")
        command([executable, PREFIX, "--nocapture"], "synthetic.log")
        command([executable, REJECTION, "--exact", "--ignored", "--nocapture"], "raw-rejection.log")
        command([executable, GENERATOR, "--exact", "--ignored", "--nocapture"], "generation.log")
        proof_path = output / "proof.json"
        log.write("tests complete; ready for final identity checks and publication: " + str(proof_path) + "\n")
    # The log is closed before publication; a failed close cannot leave proof.json.
    def before_commit():
        if identity() != (revision, tree) or digest(executable) != provenance["executable_sha256"]:
            raise ValueError("source or test executable changed during generation")
        print("validated; publishing " + str(proof_path), flush=True)

    publish(pending, proof_path, provenance, before_commit)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("rom", type=lambda p: Path(p).resolve(strict=True))
    parser.add_argument("profile", choices=("debug", "release"))
    parser.add_argument("name", nargs="?", default=datetime.datetime.now(datetime.timezone.utc).strftime("%Y%m%dT%H%M%SZ"))
    args = parser.parse_args()
    run(args.rom, args.profile, args.name)
