# Qualify the native Pandora route and state changes

## Summary

Identify and reproduce the minimum original-game path from the accepted fresh house progression to opening Pandora’s Box and regaining player control. This is a source/reference evidence gate, not portable implementation.

## Dependencies

- [Qualify the room B conversation and exterior progression](qualify-house-conversation-progression.md)
- [Qualify the first exterior landing profile](qualify-house-exterior-profile.md)

- [Make oracle video publication deterministic and race-free](fix-oracle-video-publication.md)

## Requirements

- Audit existing opening scenarios and any historical desync before treating their later state as evidence.
- Use the owned Japanese ROM, a fresh empty-SRAM Session and actual inputs from the menu/intro onward. No warps, state restoration or memory patches in the qualifying journey.
- Record exact required maps, exits, interactions, request/choice boundaries, event changes and immediate player/world effects. Separate necessary story steps from incidental movement/presentation.
- Pin relevant source bytes/operands and retained semantic checkpoints; source metadata must not initialize production state from captured WRAM.
- Explicitly record synchronization effects of observations such as save_state, timing policies and any oracle/tooling blocker.
- Keep raw ROM/text/graphics/traces/captures ignored; commit only reconstruction tooling, selected source metadata and hashes.

## Acceptance Criteria

- A reproducible input-only fresh route reaches a named post-Pandora player-control checkpoint.
- The prerequisite and effect contract is source-backed, with controls distinguishing missing prerequisites and reordered/omitted interactions where practical.
- A checker rejects mutated semantic evidence and an independent replay/review confirms the declared scope.
- Required implementation/asset boundaries and unresolved blockers are documented before the portable sequence is enabled.

## Notes

- Parent: [Open Pandora’s Box in the portable slice](open-pandora-portable-slice.md).
- RE discovery may precede executable tests; retained evidence/checker development must still demonstrate red → green mutation controls.

## Qualification in progress

- Authorized follow-up: migrate Pandora to the reviewed synchronous-video observer epoch after independently auditing parent fixed twins against all old artifacts/logs. Preserve old epoch evidence explicitly; change only verified pixel fixtures and source-authenticated observer policy/provenance. Audit older wrappers without claiming their gates renewed; no runtime/assets/host edits or shared tracker changes.
- Isolated source task owns only this detail, `tools/pandora-qualification/` and `docs/pandora-progression.md`; parent owns tracker closure and portable integration.
- Start from the accepted conversation route's 77 commands, preserving every synchronizing capture, omit its finish command, then extend the same empty-SRAM process. Independent replay must preserve that observation schedule.
- Historical scenario inventory established no accepted Pandora endpoint. The new fresh route, not those scenarios, now reaches C → E → 20 → 21 and the forced box interior tour.
- [Source/reference contract and reproduction](../../docs/pandora-progression.md): direct map13 result1 → `$28`, C result1 → `$2E`, two actual pot hits → `$292`, post-warning re-approach/polling gate → `$22`, mandatory 41 → 44 → 42 → 43 → 41 tutorial → `$243/$244`.
- Named neutral-stability/control witness: `pandora-tour-control`, completed41344, map41 `(136,208)`. The preceding `tutorial-052` at completed41224 already retains completed-tour control. Left/Up and neutral controls finish at completed41788 `(120,192)`. Later field return, `$21/$23/$FE`, equipment acquisition and combat are not claimed.
- Original source-only projection and final source/evidence/architecture reviews approved the declared scope. The two original fresh processes completed without restore/patch/seed; their old-epoch retained evidence checkers passed normally and under `-O` before migration; those references are now archived. Disabling semantic validators causes 34 expected mutation failures before restoring green.
- Parent replay `replay-JmCgU8` passes `report()` including the semantic validator in both checkouts: the full frame log, every native-state field and every non-pixel artifact match the original exactly. Only two selected pixel hashes fail final exact equality; exhaustive comparison additionally finds two unselected pixel differences. This is actual independent progression/control confirmation, **not** strict whole-capture acceptance.
- Follow-up fresh batch `diagnosis-gAPhmb` with the current parent executable again has identical log/non-pixel artifacts but three different pixel failures, including an accepted-prefix checkpoint. Independent source inspection identifies an unsynchronized worker clearing/refilling the shim framebuffer while the oracle copies it; same-binary nondeterminism supports this capture-race diagnosis, not metadata-only build drift. See the contract's replay diagnosis and `tools/pandora-qualification/replay-diagnosis.json` for every mismatch and provenance limits.
- Independent static correctness/architecture review approved this bounded diagnostics/diagnosis follow-up, not full route acceptance. Its two small test suggestions were applied: report two checkpoint pixel mismatches together and explicitly reject semantic-success wording in the prefix error.
- Historical diagnosis commit `5b7b88b` improved strict checker diagnostics only and left the visual gate blocked on a separately owned observer fix/replay/review. It changed no source prerequisites, assets or portable integration; a no-op exact comparator then caused 42 expected mutation failures.

## Synchronous observer renewal

- Separately reviewed video fix `0db0aad` is cherry-picked here as `51972b0`. Parent fixed twins `replay-WKFf0d/{a,b}/journey` are byte-identical across all 2682 artifacts/full logs and to the video sibling fixed root. Independent source-task comparison confirms only 171 pixel files differ from the original; all 383 sets of non-pixel surfaces, full logs, recipes and input/save schedules are unchanged. Parent `old-comparison.json` was also reproduced exactly against its actual old side, `replay-JmCgU8` (174 pixel-only differences).
- [Migration and wrapper audit](../../tools/pandora-qualification/OBSERVER-MIGRATION.md): 106/213 selected main pixel pins and 4/31 embedded prefix pixel pins renewed; explicit epoch/completed-frame policy and nine source hashes include build.rs, unity, patched ares.hpp, shim, API, Screen/PPU and serializer. Original main/discovery/prefix references are byte-preserved under `epochs/threaded-video-v0`; no backward pixel equality or historical binary provenance is claimed.
- Strict main checker passes normally/optimized on both parent roots and both video-sibling roots, with unchanged gameplay/source semantics. `migrate.py` enforces complete old/fixed/twin/sibling invariants before allowing pin writes; ordinary `check.py --record` is removed. The AE50 prose correction is incorporated here without touching the parent's uncommitted file: two actual warning boundaries, no source/native metadata change.
- Discovery is historical, not renewed; current `--discovery` fails explicitly. Standalone conversation/exterior fixtures remain old even though Pandora's authenticated embedded prefix is renewed. Other house/new-game/initial wrappers require their own documented gates. Parent's map-inspector RGB integration failure remains red and unrepinned: its separate SRAM fixture needs old/new full WRAM/VRAM/CGRAM and repeated fixed RGB evidence.
- New epoch/invariant tests followed red → green; semantic, exact-surface/provenance and observation-policy mutation coverage remains strict. Independent static architecture/policy review approved; its focused alias/direct-guard test suggestions were applied. Independent execution/evidence review approved the final staged code and fixtures: all six roots/complete comparison reports and archived Git bytes verified; migration normal/optimized and strict checks on all four fixed roots pass. Source/checker/epoch suites pass 6/13/11 tests each in both modes; no-op exact equality yields 49 failures, and targeted alias/full-log/non-pixel guards fail their negative controls when disabled. Old-main, discovery and `--record` rejection were verified in both modes. Publication tests pass normally/optimized and repository safety passes. Tracker validation has the inherited video-fix detail/index/roadmap mismatch from `51972b0`; shared tracker reconciliation remains parent-owned. Parent will rerun the strict checker after cherry-pick and owns acceptance/closure. No runtime/assets/host or shared tracker indices were edited.

## Parent acceptance

- After cherry-picking the reviewed epoch archive/migration, parent reran the exact strict checker on both independently produced fixed roots normally and optimized: four passes, including authenticated prefix, source operands, semantic validator and exact selected surfaces. Retained outputs: `local/oracle-video-qualification/replay-WKFf0d/strict-{a,b}-{-B,-O}.json`; no mismatches suppressed.
- This supersedes earlier pending strict-replay/observer handoffs in this issue. The input-only direct route, observation/save schedule, completed41344 control/stability witness and final two-axis controls are accepted as documented. `tutorial-052` already has completed-tour control at41224; neither label claims an exact first control-return cycle.
- Historical discovery controls remain explicitly unrenewed; equipment, field return/frozen town, tower and combat are excluded. All portable movement/admission/event/rendering work remains in the parent integration issue.
- Source/checker/epoch tests pass 6/13/11 normally and optimized. Independent evidence/migration reviews plus parent closure audit approve the declared source scope. Issue archived, not the runtime milestone.
