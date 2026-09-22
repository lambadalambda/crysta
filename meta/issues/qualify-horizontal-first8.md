# Qualify native horizontal first8 dispatch

## Summary

Extend horizontal Open/8 evidence to actual Left and Right first-sample type8
contacts in the ordinary passive resolver.

## Dependencies

- [Directional collision qualification](qualify-crysta-directional-collision.md)

## Requirements

- Input-only empty-SRAM native boot sessions; no warps, patches or save states.
- Prove actual first8 dispatch, including tentative sampling, dynamic-bit state,
  old-edge precedence and player-only PCs; distinguish aligned/unaligned cases.
- Pin contiguous replay windows and recipes; add failing coverage tests and
  mutation controls before admitting evidence. Keep production conservative.
- Unsupported or unreachable cases remain explicit, not relabeled as qualified.

## Acceptance Criteria

- Left and Right native first8 witnesses replay without exclusions or state resets.
- Relevant source branches and qualification limits are documented; regression
  gates and independent review pass.

## Notes

- Related: [horizontal pair cases](qualify-horizontal-type8-pairs.md) and
  [slope-mediated cases](qualify-slope-mediated-type8.md).
- Raw ROM/layers/traces stay ignored; publish tooling, recipes, selected metadata
  and hashes only. Native modes outside ordinary walking remain out of scope.

## Bounded native discovery result

No direct first8 witness was found in either direction. Input-only empty-SRAM
approaches to the town patch from south/east stop at `(536,320)` / `(632,246)`;
cap-side approaches climb to y176 instead. A native jump reaches y196 with
special4, then falls to y208 before ordinary recovery: it is not walking evidence.
These are route-specific blockers, not a proof of global unreachability.

The east-wall negative capture retains all142 motion frames12696–12837 after
12 settled neutral frames, with ordinary entry flags/special and passive recorded
controls. Frame12704 attempts Left−1 from `(632,246)` and samples `(38,14)` /
`(38,15)`, raw `1c44/1c49`, both type14. Its player-only dispatch is
`DB45→DB5C`, `DB63→DB86`, not first8. Capture SHA-256:
`9cbcdaf7888abd97a9bf1ea2f52c2fd4c0e3b419733e1e77dba3038f557cc1b6`.

Existing apparent Left first8 frames13401/13433/13477 divert through old raw6,
align Y and redispatch the next row as type0. Shared Open-table PCs are not
sufficient to identify type8. Neither these nor the negative wall capture are
added to the accepted replay windows.

Ignored evidence/recipes are retained under
`local/type8-discovery/first8/` (`REPORT.md`, `SHA256SUMS`). No implementation
change arose from this discovery pass, so TDD does not apply to these captures.
Both directions, alignment variants and dynamic-bit variants remain open;
other maps/states and action interactions were not exhausted.
