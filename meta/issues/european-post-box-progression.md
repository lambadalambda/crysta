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

- The portable `World` single-route test passes, but the frozen-return player's `COP DF` scripted walk is not yet modelled; Ark's position at control return differs from native. The reporter confirmed the **browser** frontend; its build/version and exact input sequence are unknown.
- The room Elder (`$0B`) and post-return mission Elder **at the `$0D` doorway**, around `(120,704)`, are different actor encounters. The native and portable direct routes talk to the doorway Elder: `$21` is set at the start of his conversation; `$296` follows the answer and remaining pages, then town `$3C`. A room Elder's presence can change on re-entry according to `$21 XOR $27`; this may explain the reported disappearance but has not been reproduced through a European player host. Confirm must be a new press after each page finishes typing; a held button does not answer a later choice.
- A fresh continuous `World` bedroom→Box→spear route now forks *only after* the on-foot frozen-return landing at `$0C (184,368)`: it walks to a frozen `$10` neighbor and reads a real page, returns through the opened B door, observes the room `$0B` Elder **absent** with `$27` set and `$21` clear, then reaches the distinct `$0D` doorway Elder at `(120,704)` on foot. His new confirm grants `$21`; answering the mission grants `$296`, the town scene grants `$3C`, and the detour reaches the world map. This supports a room-Elder-vs-doorway-Elder location mix-up but does **not** explain the player's exact browser report: the detour fork is portable `World`, not a fresh browser-host or native detour, and the reporter's saved state/build is unknown. The input-only native/portable direct routes do not substitute for this missing host evidence.
- A headless European browser session authenticated and started in the bedroom with no page error; this does **not** qualify the post-Box detour or replace a fresh host-level route. Its keyboard action edge can now survive a keyup+keydown between PAL frames; missing presses were verified, but not established as the cause of the reported Elder behavior.
- A headless **browser-host** route through the same frozen-resident/`$0B` detour remains untested. Neither `World` route proves that the reported browser interaction works.
- Related: [European native route](european-route.md), [frozen return](frozen-return-mission.md).
