# Give the darts their damage and Ark the burn status

## Summary

`$110`'s wall darts and the burn status ("Ark's toasted!", `$85:DAC3`) are not modelled.

## Dependencies

- [Play towers 2 and 3](towers-two-three.md)
- [Play tower 4](tower-four.md)

## Requirements

- Check pose `$14` of `$B0:FAF2` for an attack box; apply the darts' damage.
- Give Ark the burn status (`$0670,X`, `$84:D32B`, `COP DF $84:DBF1`).

## Acceptance Criteria

- The darts and the burn match the longplay.

## Notes

- Research: `docs/tower-three.md` §7, `docs/tower-four.md` §2.
