# Returning from the house room to Ark's bedroom

This bounded **saved-game** witness qualifies the reverse `$0010 → $000F`
doorway, not fresh startup or a general transition scheduler. It complements
[the outbound doorway](opening-doorway.md) and [fresh startup](new-game-bootstrap.md).
No RAM patches, forced warps or snapshot restores are used.

## Reproduce

```sh
sh tools/movement-qualification/house-return-replay.sh
```

The runner authenticates the private JP ROM and supplied SRAM, builds the shared
standalone probe, and runs two independent processes. Both reproduce the whole
350-row stream and ten WRAM checkpoints byte-for-byte; five native instruction
stops are separately checked. `house-return-pins.json` commits selected metadata
and hashes only. Raw outputs remain under ignored `local/movement/`.

Inputs are the saved slot-1 preamble (Start400..408, Up900..908, Up950..958,
A1100..1112), Left1601..1657, Down1657..1682, then **Up1801..1820**; all
ranges are half-open input labels, applied before `run_frame`. Completed frame
N follows N calls. This is not an ordinary Up activation during arrival: the
replay waits until the room has settled at completed1801.

| Completed | Map | Position | Ownership |
| ---: | --- | --- | --- |
| 1801 | 10 | 392,353 | ordinary idle |
| 1815 | 10 | 392,336 | last resolved walking step, resume84A367 |
| 1816 | 10 | 392,335 | transition, flags1411/resume84B9AE |
| 1831 | 10 | 392,320 | departure |
| 1832 | F | 392,319 | map switched; old player still present |
| 1842 | F | 392,208 | new player initialized, flags4414 |
| 1871 | F | 392,191 | recovery, resume84A318 |
| 1872..1950 | F | 392,191 | stable ordinary idle; gates clear at checked WRAM points |

The 1815 Up stream attempts −2 but collision resolves only −1, landing exactly
on the exit. The next frame is **not walking**, despite another one-pixel delta.
The 64-byte event block stays at SHA-256
`bf7d61f4953777ab96d024578f0a147a6700c9a75085c45454f9aec69e801fcb`
at all ten checkpoints. This does not qualify arbitrary event histories.

## Decoded bounded source

- Map10 first exit: tile(24,20), size1×1, destinationF, mode0, selector6,
  raw destination(384,176). Ordered coarse/fine selection requires anchor(392,336).
- Departure entry at `$8D:895B + (6−1)*3 = $8D:896A` contains `$84:B979`. It schedules player
  `$84:B9A7` using `COP CB`; controller `COP C1 $0010` supplies the same
  endpoint counter as the outbound route. Player `COP84 $01,$09,$01`, then
  `COP8E` loop, is the observed negative-Y translation (resume84B9AE).
- Signed adjustment `$8D:8985 + 6*4` is `(0,+16)`. Raw384,176 becomes
  queue384,192. The already-decoded `$80:F7F3..F815` player override adds
  `(8,16)` → **392,208**, not the ordinary bedroom record location.
- MapF actor pointer `$83:801E → $83:8D1E`, skip the two-byte prefix;
  FD player record `$83:8D20` points to header `$84:A129`.
- Arrival word at `$84:87FE + 6*2 = $84:880A` contains `$BB74`. It schedules player `$84:BBA2`,
  same negative-Y COP84/COP8E loop, and uses `COP C1 $0010` before restoring
  common control. Seventeen logical −Y steps give **392,191**.

Separate fresh diagnostic boots reach `$8D:8797` at392,336 before exit selection,
then `$84:B979` and `$84:B9A7` with the same anchor. A boot to1849 reaches
`$84:BB74` and `$84:BBA2` at392,208. Failed earlier diagnostic starts missed the
trigger after completed1815 and are not qualification evidence. A separate
stop at `$80:F7F3` observes ordinary record coordinates312,112 **before** the
queue override; they are not the final doorway spawn.

## Portable policy boundary

The reverse counterpart of the explicit semantic doorway policy is:
17 logical −Y steps from392,336 to392,319; one atomic map/spawn update392,208;
17 logical −Y steps to392,191, then fresh walking admission. Native arrival has
loader/scheduler stalls; these are endpoint-qualified logical updates, **not**
reference-video timing. Resetting input history after transition is preview
policy, not evidence about native cross-transition double-tap history.

Only this pair of internal doorway handoffs is covered. Other map10 exits,
NPC/dialogue interactions, outdoor maps and the rest of the house are not
silently admitted by this witness.
