# Free the souls the towers' marked enemies hold

## Summary

An enemy whose record sets `7F:1018` (`$80:F56F`) does not explode when it
dies. The death script (`$85:E2AC`) plays sound `$37` and the helper art's
burst (list `$17`, 40 frames), then leaves a soul: a blue ball (list `$13`)
that can be talked to (`+$06` = `$0200`), and wakes it (bit 13) at its
`7F:1018` script (`$102`: `$90:90B0`, a text callback, then the loop
`$90:FB36`). The runtime explodes these enemies and rolls a gem. Native
trace (JP, `$102`, 2026-10-18): death 60497, burst 60514-60553, soul from
60554, its script from 60555.

Marked enemies: `$102`, `$103`, `$108`, `$109`, `$118`, `$12E`.

## Dependencies

- [Fix the walkthrough's findings](walkthrough-findings.md)

## Requirements

- [x] Play the burst and leave the soul instead of the explosion and the
  gem; a group member's death (`$85:E353`) leaves none.
- [x] Run the soul's script; talking to it shows its text, which heals 50
  (`$DD $04CE`, `$85:96A9`).
- [x] After the talk, the souls go: a child (`$90:FB59`, list `$15`) sends
  the soul to list `$14` and both go (`COP A7`). Not timed natively.
- [ ] `$102`'s soul instead pans the camera to the gate and opens it
  (`$90:90CD`: flags `$282`/`$283`, `COP DF`, `COP DC`, `7F:201A`, `COP 46`).
  It stops at `$90:90DE`.
- [x] The soul stands in Ark's way (its cell; natively Ark stops 2 pixels
  sooner, at y 402 below `$102`'s soul).
- [ ] Sound `$37` with the burst.

## Acceptance Criteria

- On `$102`, killing the marked blob shows the burst for 40 frames, then
  the soul, on both ROMs; no gem drops; talking to the soul opens its text.
