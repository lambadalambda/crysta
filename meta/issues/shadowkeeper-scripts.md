# Run Shadowkeeper's scripts natively

## Summary

The body (`$93:D876`) freezes at `$93:E767` (`STA $04AA`); the stand-in holds it as a two-life enemy. The native fight has a head, a claw holder and two claws, a tail of 11 segments, shots, a sting, and two stages (`docs/tower-five.md`).

## Dependencies

- [Play Shadowkeeper's full fight](shadowkeeper-full-fight.md)

## Requirements

- Spawns `9A`, `A0`, `EA`; services `04`, `5A`, `63`, `6F`, `A8`, `AF`, `E4`.
- Native code: the phase words `$04A6`/`$04A8`/`$04AA`, `$049A`, `$0DEC`, `INC`/`STZ $0498`, `EOR $04A4`; reads of `$0812`/`$0822`/`$081E`, `$0958`/`$095A`; the `+$2E` list walk; writes into other actors' `+$0A`/`+$0C`/`+$0E`, x/y, `+$04`, `+$14`/`+$16`; the death callback `$7F:1012`; `JSL $86:81B0`.
- Remove the stand-in.

## Acceptance Criteria

- The fight's phases, attacks and deaths match the native trace; no script freezes on `$123`.

## Progress

- Done (2026-10-09): the scripts run with Rust models of their native code (`docs/tower-five.md`, "Shadowkeeper's fight in the runtime"); the stand-in is gone. The phases (`$B`, `$C`, `$E`, `$11`), the shots' 20 damage, the claws' and the body's lives follow the native trace; frame-exact timing is not comparable (the trace has lag frames).
