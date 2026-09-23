# Play the Crysta story from the wake-up scene to the world map

## Summary

Make the native app a first slice of real gameplay: a new game starts with
Elle waking Ark, the player does everything the story requires in Crysta,
and walks out of the south gate onto the world map. Scenes run from the
game's own scripts, not hand-authored graphs.

## Dependencies

- [Make the Crysta slice fully playable](playable-crysta-slice.md)

## Requirements

- Execute the ROM's actor and scene scripts for every step on the critical
  path; hand-written behaviour only where a script service is understood
  and documented.
- Verify each step against the continuous input-only native route
  (`new-game-qualification` → `house-conversation` → `pandora-qualification`
  → `tower-approach-qualification/tower-route.jsonl`), frame 0 to 59760.
- Keep free roam, residents and timing as qualified today.

## Acceptance Criteria

- A deterministic headless replay goes from new game to the world map using
  only player inputs, passing every flag on the path in order.
- The same inputs are playable in the native window.
- Each sub-issue below is closed with its own evidence.

## Notes

- Milestone: [M4 — Portable vertical slice](../milestones.md#m4-portable-vertical-slice).
- Related: [portable event runtime](portable-event-runtime.md),
  [actor script VM](decode-actor-script-vm.md).
- Critical path (map, flag): wake-up F `$20`; Elder B `$26`; weaver `$13`
  `$28`; blue door C `$27`, `$2E`; pot hits `$292`; box `$21` `$22`; tour
  `$41..$44` `$243`, `$244`; spear `$240..$242`; frozen return `$FE`, `$23`;
  doorway Elder D `$21`, `$296`; town scene `$3C`; south gate to `$03`.

## Sub-issues

1. [Run scene dialogue, choices and flags from scripts](script-dialogue-choices.md)
2. [Play Elle's wake-up scene](bedroom-wake-up-scene.md)
3. [Apply flag-gated doors, blockers and tile patches](flag-gated-geometry.md)
4. [Run scripted movement, entry scenes and map transfers](scripted-movement-scenes.md)
5. [Lift and throw pots to break the blue door](pot-throw-door.md)
6. [Open the box in the cellar](cellar-box-sequence.md)
7. [Play the tour inside the box and take the spear](box-tour-and-spear.md)
8. [Play the frozen return and the Elder's mission](frozen-return-mission.md)
9. [Leave through the south gate onto the world map](south-gate-world-map.md)
