# Archived threaded-video-v0 evidence

These JSON files are byte-preserved pre-migration evidence, **not** current
synchronous-video acceptance fixtures:

- `reference.json`: original fresh Pandora main journey.
- `discovery-reference.json`: separate fresh refusal/retry/negative-control journey.
- `prefix-reference.json`: original house-conversation reference, copied without
  changing the standalone house-conversation fixture.

The main/discovery references came from source-task commit `5b7b88b`; the prefix
is the unchanged conversation reference at that commit. `migration.json` two
levels above binds all three archived file hashes. The original checker/source
is available at that commit; no current-source hash is substituted for the old
producer's provenance. The old producer executable/compiler identity was not
retained. See `docs/pandora-progression.md` for the threaded capture race.

Current strict acceptance uses `headless-sync-video-v1`. It does not promise
backward pixel equality. `migrate.py` authenticates the old main checkpoints and
prefix against these files and compares every artifact/full log to independently
reproduced fixed captures before permitting pixel/policy migration. No migration
command overwrites this archive.

Discovery has **not** been replayed/renewed under the fixed policy. Its source and
semantic negative controls remain historical evidence, exercised by ROM-free
mutation tests. `check.py --discovery` deliberately reports this remaining gate
rather than treating old or newly captured discovery data as current-qualified.
No flag bypasses the current main-route pixel or observer checks.
