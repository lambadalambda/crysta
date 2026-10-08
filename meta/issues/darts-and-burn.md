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

## Progress

- Research: `docs/darts-and-burn.md`.
- The darts hurt: the launchers are hittable records of profile 5 whose pose packet the art decoder refuses; the runtime made enemies only of bodies. Now any hittable record with a profile attacks.
- An enemy's hit follows `$85:D648`: the attack kind (`7F:102C`), Ark's type words (`$064E`..`$0654`), the status roll with the armor table (`$8D:BD92`), the enemy critical (element 13) and the immune path (`$85:D70C`). The burn runs (`world/status.rs`): 120 frames, the script holds Ark, "...TOASTED", 60 frames out of reach; a map load clears it (`$85:DFA8`).
- Open: the other statuses' runners (sleep, element 10, and the rest: no underworld enemy gives them); a native trace of a dart hit and a flyer's burn.
- Done: native JP traces of a dart and a flyer's burn (`docs/native-warp.md`) match: the dart's push (stream `$38`, list `$0E`, 43 frames out of reach, hits at h, h+43, h+198), the hurt sound on port 2, and the burn's frames (script at h+1, list 4 from h+2, 5 from h+59, Ark free at h+123). Other statuses have no underworld source.
