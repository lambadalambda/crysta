# Room B conversation → first exterior: source contract

Scope: Japanese ROM SHA-256
`f331e3941e595cc41e26968c20b6e31563ad19603e5e204d93e3ee2e22344548`.
This is semantic progression qualification, not a native scheduler, general event
VM, dialogue/font decoder, or exterior graphics/collision implementation.
[Tracked issue](../meta/issues/qualify-house-conversation-progression.md).
RE/source evidence substitutes for initial red tests. Only source metadata,
hashes, tooling and input commands belong in Git; generated evidence stays local.

## Core contract (source backed)

**A flat list of pages followed by one completion flag is insufficient.**
The fresh request grants the flag before a subsequent choice and follow-up.
Use stable resident identity `$838B96`, not its native allocation slot `$1040`.
Its callback is `$888EDE` (the write at `$888F08` is not the callback entry).

| Phase | Source request / control | Effect |
|---|---|---|
| B entry | `$888E6E → $888FDA`, wait `$888E72` | No `$0026` grant; do not confuse arrival text with interaction |
| First interaction | `$888F02 → $888FF0`; wait `$888F06` | After this request returns, `$888F08 COP07 $8026` sets global `$0026` |
| First choice | `$888F0C COP1A`, catalog 0 | Two options; cancel result 0, option results 1/2 |
| First option 1 | `$888F1D →` resident continuation `$888E86 → $8890D9` | Follow-up text, no additional global event write |
| First option 2 or cancel | `$888F17 → $888E96 → $88905A` | Alternate follow-up, no additional global event write |
| Repeat | `$888F23 → $889156`; wait `$888F27`; choice `$888F29`, catalog 1 | `$0026` already set |
| Repeat option 1 | `$888F34 → $88918C` | Follow-up, no additional global event write |
| Repeat option 2 or cancel | `$888F3B → $8891D6` | Alternate follow-up, no additional global event write |

The callback first tests `$0109,$003B,$0296,$0021,$0028`, then `$0026`.
Only the fresh-house state (`$0020,$00FB`, later plus `$0026`) is qualified here;
other story branches must not accidentally fall through to this transcript.
`$003B` targets the immediately following check rather than changing this path.
Repeat selection at admission is safe for this bounded state. Page/choice data
must come from the separately owned dialogue decoder, not copied Japanese text.

`COP1B` publishes a banked request; `COP1F` executes/waits for it to finish.
`COP1A` calls `$859F28`, then dispatches an inline result table. It is a real
choice, **not an animation/facing selector**. The first choice table at `$888F11`
is `[cancel:$888F17, option1:$888F1D, option2:$888F17]`; the repeat table at
`$888F2E` is `[$888F3B,$888F34,$888F3B]`.

### Admission and controls

- Native interaction registration at `$848A1E` is held **A** (`$0080`), tested by
  `$80906F..90AC`; not exclusively a rising edge. The dialog/UI can impose its
  own deterministic input admission without reproducing CPU frame counts.
- `$87923F` refuses while `$0DC2 != 0`, then dispatches by facing `$0956`.
  Actor targeting `$87C783..C7F0` precedes tile fallback. It walks the list rooted
  at `$0DFA`, following entity `+$2E`, taking the first rectangle hit among actors
  with `+$04 & $0100`. Callback lives at extended `+$20`, installed by `COP21`.
- Candidate rectangles use extended offsets `+$28/+2C` and extents `+$2A/+2E`;
  unsigned 16-bit differences are accepted **inclusively** (`<= extent`). The
  ordinary resident witness has origin `(120,112)`, offsets `(-8,-16)`, extents
  `(16,16)`: rectangle **X112..128, Y96..112**.
- Forward sample points are 8 and 16 pixels along facing from Ark's position.
  Up at `(120,128)` hits the lower rectangle boundary; Down there misses. The
  first actor-hit path additionally checks callback/interaction-policy bits
  `entity+$06` (`$0200` unrestricted, otherwise `$0100` with opposite facing),
  and optional gate fields at `$7F:2028+slot` / `$7F:2026+slot` before dispatch.
  Here “extended +offset” means `$7F:0000+slot+offset`; ordinary entity fields
  are `$7E:0000+slot+offset`.
  The fresh idle resident witnesses `$0200`, not an opposite-facing restriction.
- Native tile type `$2000` can extend the **far** sample another 32 pixels along
  facing, after ordinary actor/tile probes fail. This is a counter interaction
  rule, not permission to replace targeting with a radial distance check.
- Choice handler `$859F46..9FE6`: Up/Down/Left/Right follow catalog neighbor links;
  A or L (`$00A0`) confirms; B (`$8000`) cancels, returning result 0. The two
  required catalogs link Up/Down between options, with no Left/Right neighbor.
  Text-page A/L acknowledgements are separate from B cancellation; native B/X
  pulses did not advance the witnessed follow-up text.

### D gate: load-time membership, not a live event subscription

Hidden gate **`$838CC8`**, header `$88A9AF`, origin `(120,720)`:

1. `$88A9B4 COP48 $8026`: reject/delete if global `$0026` is already set.
2. `$88A9B8 COP3B`: otherwise stamp occupancy at cell **1415** (`7,44`),
   raw `$0592 → $8592`, independent of the visible wandering resident.
3. `$88A9BA COPBC`: save continuation `$88A9BC`, a bare RTL.

`COPBC` handler `$80AAA5..AAB2` stores the continuation and returns; later actor
invocations return immediately. There is **no retest, COP3C clear, or unlink**.
Changing `$0026` while D is already loaded does not remove this stamp. Reloading
D reconstructs occupancy and rejects the actor when the flag is set. A normal
B-conversation→D journey necessarily loads D after the grant, so the gate is
absent. This already-loaded counterfactual is source-qualified, not a RAM-poked
acceptance experiment. Existing pre-conversation closed-gate native evidence is
in [house backgrounds](house-backgrounds.md) and [scene census](house-scene.md).

### Outgoing endpoint

The first exterior is **map `$000A`**, not an unverified name guess. D's direct
exit `$818DFE` has rectangle `(7,44,1,4)`, destination A, mode0, selector5, raw
position `(496,752)`. A's selected player record is `$8389B8`; its source record
position is **not** the exit's queued/settled spawn. Existing shared exit logic
uses bounding origin `Ark+(-8,-16)` and first coarse match followed by fine
exclusive deltas; D's exact X alignment is Ark X120 and eligible Y720..768.
Selector5 queues raw destination plus `(0,-16)`, then loader/arrival scheduling
produce the actual position; do not teleport directly to the record's spawn.
Native spawned/settled/walking endpoint qualification remains in progress and
is outside this initial source contract.

## Source reproduction

```sh
python3 -B tools/house-conversation-qualification/source.py \
  'local/Tenchi Souzou (Japan).sfc' > local/house-conversation-source.json
```

`source.json` pins bounded code/data investigation windows and typed source
operands, not raw scripts or dialogue. `source.py` is deliberately not a general
actor interpreter. The separate dialogue task owns requests, page boundaries,
choice text/layout and font decoding; the exterior task owns assets/collision.
