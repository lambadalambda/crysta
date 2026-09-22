# Qualify native slope-mediated horizontal type8 contacts

## Summary

Prove the remaining horizontal type8 behavior reached through slope6/7 old-edge
or new-edge paths, separately from ordinary first/second table dispatch.

## Dependencies

- [Directional collision qualification](qualify-crysta-directional-collision.md)

## Requirements

- Source-ground the applicable Left/Right slope paths, raw type8 probe semantics
  and crossing/alignment redispatch; do not mirror asymmetric native branches.
- Capture genuine input-only boot sessions with no warps, patches or save states.
- Prove the relevant type8 read and player-only source path, not nearby8 or NPC
  coverage. Pin contiguous windows, recipes and coverage/mutation controls.
- Preserve ordinary/passive admission and conservative production. Do not remove
  unexplained frames or replace the walking state between captured frames.

## Acceptance Criteria

- Applicable horizontal slope-mediated type8 cases have bounded native witnesses
  and passing replay, or missing cases remain explicitly open with blockers.
- Existing regressions pass; independent source/correctness review completed.

## Notes

- Related: [first8](qualify-horizontal-first8.md) and
  [ordered pair cases](qualify-horizontal-type8-pairs.md).
- Raw ROM/layers/traces remain ignored; only tooling, recipes, selected source
  metadata and hashes are committed. No production enablement is implied.

## Bounded source/native discovery result

All slope8 classes remain **unqualified and open**. The audit covered22 prior
captures (15,029 motion rows;5,733 horizontal attempts, including repeated
routes), plus three new input-only boots. No actual raw8 slope probe,
crossing→8 redispatch or slope-promoted Partial-table8 witness was found.
This is not an exhaustive map/state or reachability claim.

Important source distinctions retained for future discovery:

- Left old6 retains the unconditional below probe; Right old7 skips it.
- New unaligned Left7 probes above at `$DC18`, while Right6 probes below at
  `$DF94`. Both can promote to the Partial table, but actual second-cell8 and
  `$DB59→DB7B` / `$DECF→DEF1` dispatch must be proved separately. On Left the
  above probe is not the second sample; on Right bit15 can override that sample.
- All four old6/7 crossing paths align Y and clear the second-sample flag before
  single-sample redispatch. Nearby8 at the pre-alignment row is insufficient.
- Left6 `$DBD9` and new Right7 `$DF55` repeat below reads already tested zero;
  they are not additional independently applicable raw8 branches on stable layers.

Fresh West frames14098–14375 (278 contiguous) have ≥12 settled neutral prefix
frames. Frame14188 moves `(343,177)→(344,176)` through Right old6 crossing,
but the aligned lookup reads `(21,10)=0026`, type0, not row11 type8.
West capture SHA-256:
`a6807cc04744a7d8b539bafae0ecddc01599da73857b8ba922e34ff78bfb7005`.
Diagonal discovery frames13210–13417 (208) also find no slope8; SHA-256:
`0602e724317519518c516b4d489ab69473fcccdf2bcc84f4ff414f081e03dc63`.
The third, cap experiment is explicitly unsuitable for qualification: only11
fully settled prefix frames. No new capture is admitted to replay.

Entry flags/special and both recorded controls satisfy the existing ordinary /
passive masks. The observer does not independently record post-hook flags or
special; no stronger admission claim is made. Native control paths are restricted
to the player's horizontal segment through its first `$D155`, excluding later
NPC execution. No implementation arose from this discovery pass (TDD N/A).

Ignored recipes, captures, branch inventory and hashes are retained under
`local/type8-discovery/slope8/`; see `slope8-handoff.md`. Future work needs a new
reachable layout/state, not relabeling type0 controls as type8 qualification.
