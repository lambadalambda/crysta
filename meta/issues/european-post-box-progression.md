# Reproduce European post-Box Elder and frozen-town progression in the player host

## Summary

A player reported that after Pandora's Box the Elder at his house did not respond; after speaking to frozen residents and returning, he had disappeared/reappeared in his room, the expected scene did not happen, and choosing to save the villagers had no effect. Determine whether this is an older build, an unmodeled player-script/arrival state, a host input problem, or a current progression bug.

## Dependencies

- [Accept the European ROM in the app and the web page](european-hosts.md)

## Requirements

- Trace the player's on-foot route through the Box, spear, frozen return, doorway Elder, mission choice and town scene with the European ROM.
- Do not infer success from setting flags manually, entering maps directly or a seeded test position.
- Compare observable host behavior with the input-only native European route at the relevant checkpoints.

## Acceptance Criteria

- A headless host-level regression from a fresh European game reaches the world map after the spear, return scene, Elder's `$21/$296` choice and town `$3C` scene, with real inputs and no stuck dialogue or controls.
- If the player report still reproduces, fix the smallest responsible behavior and test it. Otherwise record why the original environment differs before closing.

## Notes

- The portable `World` single-route test passes, and the frozen-return player's
  exact map-`$21` `COP DF`/`COP 84` profile is now modelled. A native witness
  pins script release at `(136,464)`; portable agrees, then retained manual
  Left/Up reaches `(120,448)`. The reporter confirmed the **browser** frontend;
  its build/version and exact input sequence are unknown.
- The room Elder (`$0B`) and post-return mission Elder **at the `$0D` doorway**, around `(120,704)`, are different actor encounters. The native and portable direct routes talk to the doorway Elder: `$21` is set at the start of his conversation; `$296` follows the answer and remaining pages, then town `$3C`. A room Elder's presence can change on re-entry according to `$21 XOR $27`; this matches the current host's room-Elder disappearance, but the reporter's exact saved-state behavior is unknown. Confirm must be a new press after each page finishes typing; a held button does not answer a later choice.
- A fresh continuous `World` bedroom→Box→spear route now forks *only after* the on-foot frozen-return landing at `$0C (184,368)`: it walks to a frozen `$10` neighbor and reads a real page, returns through the opened B door, observes the room `$0B` Elder **absent** with `$27` set and `$21` clear, then reaches the distinct `$0D` doorway Elder at `(120,704)` on foot. His new confirm grants `$21`; answering the mission grants `$296`, the town scene grants `$3C`, and the detour reaches the world map. This portable `World` fork first established the room-Elder-vs-doorway-Elder distinction; the fresh host/browser detour below now follows it, but neither run identifies the reporter's original build or saved state.
- A headless **browser-host** route now replays from a fresh `Game::new` bedroom rather than seeding a map or flag: the refusal→retry branch opens the blue door, Pandora's Box tours the five rooms, and Ark takes the Crystal Spear. After the frozen return, the *same live host* walks to a `$10` frozen neighbor and reads its European page, visits the `$0B` room where the Elder is absent while `$27` is set and `$21` clear, then meets the different Elder at the `$0D` doorway `(120,704)`. His new X press grants `$21`, the mission answer grants `$296`, town grants `$3C`, and the host reaches and walks on world-map `$03` without fault or locked input. The tracked Rust test drives **only `Game::frame` inputs** and checks the native direct-route progression flags and each portable on-foot map/position checkpoint, including frozen-return release `(136,464)` and the later manual Left/Up boundaries. An owned European ROM was selected in a rebuilt headless Wasm page and its regenerated 28,121-frame test input trace replayed through the page's keyboard handlers and controlled PAL RAF loop with **no input-edge mismatches, faults or page errors**. This is not a native detour comparison (native only took the direct post-Box route) and uses synthetic input/clock rather than the reporter's original device. The reporter's browser build, saved state and exact input sequence are still unknown, so retain the issue until that environment can be compared or reproduced.
- The earlier headless European browser startup check established only ROM acceptance and an English bedroom with no page error. A missing keyboard re-press was fixed separately; neither startup nor that edge fix alone explains the reporter's Elder behavior.
- Related: [European native route](european-route.md), [frozen return](frozen-return-mission.md).
