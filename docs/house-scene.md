# Fresh house scene census (before decoder/integration expansion)

This is the **first bounded census result**, not an implementation of the whole
house. Production sprite APIs are unchanged. Source projection and two fresh
input-only reference itineraries establish the room graph, nine admitted
residents in six visited rooms, visible non-resident output, and explicit
progression/phase boundaries. **Do not equate “nine source-connected scenes” with
“nine initially walkable rooms” or “nine static NPCs reproduce native behavior.”**

## Immediate planning result

| Scene | Camera/sector | Fresh resident disposition |
|---|---|---|
| B | `(0,0)` | 1 Elder; visited; entry text differs from progression conversation |
| C | `(0,256)` | 4 residents; visited; stationary origins but multi-pose ordinary loops |
| D | `(0,512)` | 1 **natively wandering** resident; visited; hidden exterior-door blocker |
| E | left `768..1023` sector | 0 predicted on unchanged fresh flags; entry not qualified |
| F | `(256,0)` | 0 post-intro residents; 1 source-created table object; visited |
| 10 | `(256,256)` | 2 residents; visited; already-qualified first resident plus companion |
| 11 | `(256,512)` | 1 resident; visited; pose changes while origin remains fixed |
| 20 | right `768..1023` sector | 0 predicted on unchanged fresh flags; entry not qualified |
| 21 | separate first layer | phase-dependent special visual/controllers, **not three ordinary NPCs**; entry not qualified |

The two capture processes reproduce **B:1, C:4, D:1, F:0, 10:2, 11:1**. Counts
exclude Ark, Ark's shadow, hidden interaction actors, compact services, departed
intro actors and the transient scene label. Ordinary actor art uses OBJ2;
resident palettes observed here are **4 or 5** (bases 192 or 208). F's object and
Ark's shadow use palette 2 (base 160). These are resource/ordering observations,
not yet source-to-VRAM art qualification for every new family.

### Blockers and required presentation policies

- **D→A is progression-gated**, not a free exit. The hidden actor at `(120,720)`
  supplies occupancy while event `$0026` is clear. The visible resident wanders
  and warns when Ark presses Down near the doorway. Fresh captures show its
  origin changing from `(72,672)` to `(104,672)`; an earlier moving checkpoint
  also has `(104,688)`. Freezing it cannot claim to port native movement/gating.
- B's entry dialogue does **not** set `$0026`. The interaction callback sets it
  at `$88:8F08`, after its separate conversation. The itinerary deliberately did
  not complete that interaction: initial global events remain `$0020,$00FB`.
- C's blue-door approach is a **blocked frontier**, not proven entry to E.
  Walking reaches `(184,392)`; an A pulse acknowledges approach dialogue and
  local event `$0001` becomes set; further Up reaches `(184,368)` but does not
  enter E. `$0001` is not evidence that the door opened. E/20/21 need separately
  qualified access/phase work before being advertised as playable fresh rooms.
- C's four residents use ordinary selectors 4/5/3/3, rather than the one-frame
  selector 2 used in room 10. Selected compositions change without origin
  movement. Room 11 likewise changes composition during a passive 180-frame
  wait. A new decoder should expose bounded source frame lists, or label a
  selected setup pose explicitly; do not silently promise full native behavior.
- F has a visible child object and a hidden interaction parent. Its selector 42
  repeatedly presents `$A2:F0AC` in the existing 6800–6900 evidence; neither the
  parent nor the departed intro NPC should be drawn as another resident.
- D's four OBJ3 glyphs are a **temporary screen-space scene-label overlay**, not
  a permanent world-space sign prop. They disappear later in the itinerary.
- Every visited scene also emits Ark's ordinary shadow via a separate actor and
  special depth flags. It is not another NPC, and it must not be included in the
  ordinary world-Y tie sort. Other Ark action/item/UI paths remain out of scope.

## House boundary: graph, resources and access are distinct

```text
B ↔ C ↔ 10 ↔ F ──→ 122   exceptional, unqualified branch
    ↕    ↕
    D ↔ 11
    ↕
    A                    ordinary exterior boundary, initially gated

C ↔ E ↔ 20 ↔ 21          attached progression/phase frontier
```

B/C/D/E/F/10/11/20 share the **512×1024** first-layer sheet `$AF:CBB3`
(headerless `$2FCBB3`). They occupy the left/right sectors in four vertical bands;
loading that whole sheet under F does not make every sector traversable as map F.
Their loading scripts resolve through `$98:8405` or equivalent explicit first-layer
loads (`$98:8410`, `$98:8458`, `$98:85ED`). Scene 21 loads `$B2:81D1` instead at
`$98:85F9`. This extra attached scene cannot be omitted merely because it has a
separate resource. The first-background decoder currently admits only F/10 and
the intro cavern; this census **does not extend those production profiles**.

The complete source graph is reproduced in `reference.json`: nine bank-$81`
exit-table entries, twenty direct 12-byte exits and their terminators. All have
mode 0; all are direct destinations. The only outgoing destinations outside the
nine-scene scope are A and 122. A reciprocates D at `$81:8D6B` and has the exterior
loading recipe `$98:835F`, serving many other building entrances.

F's exceptional `$81:8E48` targets 122; its list then connects to 121/123, not
back to F, and uses loading script `$B3:88ED`. A separate fresh walking diagnostic
(Right 94, Up 90, wait 100 after bootstrap) ends at **F `(441,112)`**, never at
its fine-match target `(440,96)`. This is a negative witness for that itinerary,
**not proof that every action/event state prevents entry**, nor permission to
silently classify 122 as an ordinary house room.

## Resident integration contract to implement next

Use **source record identity**, not runtime slot or frame key, for instances.
Different actors can reuse one resource and pose. The proposed next API is a
`HouseScene` roster containing source ID, map ID, source spawn position, bounded
pose/list identity, facing/flips, palette base, shared composition access and an
explicit presentation/depth class. The existing `HouseNpc` API must remain a
compatible wrapper over shared loading, not become one bespoke decoder per NPC.
Compressed-packet pose keys and direct-ROM composition keys need distinct forms.

| Map | Stable source ID | Source origin | Ordinary source selection / witnessed facing |
|---|---|---|---|
| B | `$83:8B96` | `(120,112)` | selector 6; Down; entry dialogue state also observed |
| C | `$83:8C0A` | `(88,416)` | selector 4; Up |
| C | `$83:8C14` | `(56,384)` | selector 5; Right, H-flip clear; descriptor **reuse** |
| C | `$83:8C1E` | `(72,368)` | selector 3; Down |
| C | `$83:8C28` | `(104,368)` | selector 3; Down |
| D | `$83:8CB4` | `(72,672)` | randomized movement loop; selected arrival witness Up, not permanent facing |
| 10 | `$83:8D7C` | `(424,416)` | selector 2; Right, H-flip clear |
| 10 | `$83:8D86` | `(440,416)` | selector 2; Left, H-flip set |
| 11 | `$83:8DE2` | `(440,640)` | selector 4; Up, H-flip set; multi-pose loop |

F's child instance can use creation identity **`$88:D618`**, with origin derived
from source parent `$83:8D4F` plus `(0,-16)`, yielding `(472,144)`. Its direct ROM
animation base is `$A2:C000`, selector `$42`. It changes runtime allocation from
`$10C0` initially to `$1080` on return; slot identity would be incorrect.

### Generalized ordinary painter ties

Source `$80:EAE6..EC1C` enumerates `$0DFC → entity+$2C`, computes inverse-Y
buckets, and prepends equal-depth entries. Earlier OAM wins opaque OBJ overlap.
The checker independently correlates **all admitted ordinary entities**, not
just Ark versus the first NPC. For ordinary `+$06 & $7800 == 0`, zero draw
override, admitted screen Y `[0,255]`, paint in increasing world Y, then the
following increasing tie rank. These are compact relative ranks preserving the
witnessed native list order, **not slot-ID or spawn-order sorting**:

| Map | Back-to-front equal-Y order, first = rank 0 |
|---|---|
| B | `$838B96`, Ark |
| C | `$838C28`, `$838C1E`, `$838C14`, `$838C0A`, Ark |
| D | `$838CB4`, Ark |
| F | child `$88D618`, Ark |
| 10 | `$838D86`, `$838D7C`, Ark |
| 11 | `$838DE2`, Ark |

The full linked list—including hidden controllers—is retained in each generated
local census report and pinned by its digest. Controller holes do not alter
these relative ranks. D movement/warning does not justify inventing new tie
rules. Shadows and screen text are different ordering classes; no whole-scene
ordering or actual equal-Y overlap capture is claimed. The established house
first-background rule remains hardware **BG2**, mode `$09`, low BG < OBJ2 <
opaque high BG. Applying that mode/assignment to new profiles requires the
parent's separate background qualification; OBJ3 text is not an OBJ2 actor.

## Complete selected-list inventory

The actor loader selects bank `$83` after zero bank-$82` table entries, skips a
two-byte scene prefix, and processes the following source records. Positions in
these tables are raw source spawns, **not final Ark arrival coordinates**.
`FD/FE` use implicit `$9A:D000`; `FE` begins at `(8,0)`; `FB` allocates a separate
compact actor; `FF` appends automatic `$85:8008` using the following text stream.
A zero ordinary descriptor means **reuse**, not invisibility.

Dispositions: **R** admitted resident; **P** Ark; **H** hidden interaction or
controller (lifecycle noted below); **X** rejected/departed on the selected fresh
path; **C** compact service; **E** scene effect; **V** special phase visual;
**T** automatic trailing-text service. E/20/21 dispositions are source predictions
under the stated event conditions, not fabricated native visits.

<!-- Generated source inventory tables follow. -->

### Map $0B, list $838B7C

| Record / type | Header → entry | Descriptor / implicit base | Source position | Disposition |
|---|---|---|---|---|
| `$838B7E` FD | `$84A129` → `$84A12E` | `$9AD000` | (136, 208) | **P** |
| `$838B96` O | `$888E4B` → `$888E50` | `$83ED7F` | (120, 112) | **R** |
| `$838BA0` FD | `$889688` → `$88968D` | `$9AD000` | (184, 112) | **H** |
| `$838BA7` FD | `$889688` → `$88968D` | `$9AD000` | (184, 144) | **H** |
| `$838BAE` O | `$888038` → `$88803D` | `$83ED4F` | (8, 16) | **X** |
| `$838BB8` FF | `$858008` | — | — | **T** |

### Map $0C, list $838BF0

| Record / type | Header → entry | Descriptor / implicit base | Source position | Disposition |
|---|---|---|---|---|
| `$838BF2` FD | `$84A129` → `$84A12E` | `$9AD000` | (136, 352) | **P** |
| `$838C0A` O | `$88A321` → `$88A326` | `$83ED5A` | (88, 416) | **R** |
| `$838C14` O | `$88A4EB` → `$88A4F0` | `$000000` | (56, 384) | **R** |
| `$838C1E` O | `$889A6A` → `$889A6F` | `$83ED74` | (72, 368) | **R** |
| `$838C28` O | `$88A1A4` → `$88A1A9` | `$83EDF8` | (104, 368) | **R** |
| `$838C32` O | `$88AAE9` → `$88AAEE` | `$000000` | (184, 352) | **X** |
| `$838C3C` O | `$888038` → `$88803D` | `$83ED4F` | (8, 16) | **X** |
| `$838C46` FE | `$88ACA9` → `$88ACAE` | `$9AD000` | (8, 0) | **H** |
| `$838C4B` FF | `$858008` | — | — | **T** |

### Map $0D, list $838CA1

| Record / type | Header → entry | Descriptor / implicit base | Source position | Disposition |
|---|---|---|---|---|
| `$838CA3` FD | `$84A129` → `$84A12E` | `$9AD000` | (120, 624) | **P** |
| `$838CB4` O | `$88A837` → `$88A83C` | `$83EDEB` | (72, 672) | **R** |
| `$838CBE` O | `$888BEB` → `$888BF0` | `$83ED9C` | (120, 720) | **X** |
| `$838CC8` FD | `$88A9AF` → `$88A9B4` | `$9AD000` | (120, 720) | **H** |
| `$838CCF` O | `$888038` → `$88803D` | `$83ED4F` | (8, 16) | **X** |
| `$838CD9` FF | `$858008` | — | — | **T** |

### Map $0E, list $838CFA

| Record / type | Header → entry | Descriptor / implicit base | Source position | Disposition |
|---|---|---|---|---|
| `$838CFC` FD | `$84A129` → `$84A12E` | `$9AD000` | (136, 880) | **P** |
| `$838D03` O | `$88A321` → `$88A326` | `$83ED5A` | (168, 896) | **X** |
| `$838D0D` O | `$888038` → `$88803D` | `$83ED4F` | (8, 16) | **X** |
| `$838D17` FE | `$88805D` → `$888062` | `$9AD000` | (8, 0) | **E** |
| `$838D1C` FF | `$858008` | — | — | **T** |

### Map $0F, list $838D1E

| Record / type | Header → entry | Descriptor / implicit base | Source position | Disposition |
|---|---|---|---|---|
| `$838D20` FD | `$84A129` → `$84A12E` | `$9AD000` | (312, 112) | **P** |
| `$838D31` FB | `$8798E8` → `$8798EB` | — | — | **C** |
| `$838D36` O | `$889752` → `$889757` | `$83F881` | (328, 112) | **X** |
| `$838D40` O | `$888038` → `$88803D` | `$83ED4F` | (8, 16) | **X** |
| `$838D4A` FE | `$888AB5` → `$888ABA` | `$9AD000` | (8, 0) | **H** |
| `$838D4F` FD | `$88D60D` → `$88D612` | `$9AD000` | (472, 160) | **H** |
| `$838D56` FF | `$858008` | — | — | **T** |

### Map $10, list $838D69

| Record / type | Header → entry | Descriptor / implicit base | Source position | Disposition |
|---|---|---|---|---|
| `$838D6B` FD | `$84A129` → `$84A12E` | `$9AD000` | (392, 96) | **P** |
| `$838D7C` O | `$8898A9` → `$8898AE` | `$83ED5A` | (424, 416) | **R** |
| `$838D86` O | `$889991` → `$889996` | `$83ED74` | (440, 416) | **R** |
| `$838D90` O | `$888038` → `$88803D` | `$83ED4F` | (8, 16) | **X** |
| `$838D9A` FB | `$8798E8` → `$8798EB` | — | — | **C** |
| `$838DA6` FD | `$889A46` → `$889A4B` | `$9AD000` | (440, 368) | **H** |
| `$838DAD` FF | `$858008` | — | — | **T** |

### Map $11, list $838DCF

| Record / type | Header → entry | Descriptor / implicit base | Source position | Disposition |
|---|---|---|---|---|
| `$838DD1` FD | `$84A129` → `$84A12E` | `$9AD000` | (216, 432) | **P** |
| `$838DE2` O | `$88A9BD` → `$88A9C2` | `$83ECAE` | (440, 640) | **R** |
| `$838DEC` FD | `$88C88E` → `$88C893` | `$9AD000` | (360, 672) | **H** |
| `$838DF3` FB | `$8798E8` → `$8798EB` | — | — | **C** |
| `$838DF8` FB | `$8798BF` → `$8798C2` | — | — | **C** |
| `$838DFD` FB | `$8798E8` → `$8798EB` | — | — | **C** |
| `$838E02` O | `$888038` → `$88803D` | `$83ED4F` | (8, 16) | **X** |
| `$838E0C` FF | `$858008` | — | — | **T** |

### Map $20, list $83923A

| Record / type | Header → entry | Descriptor / implicit base | Source position | Disposition |
|---|---|---|---|---|
| `$83923C` FD | `$84A129` → `$84A12E` | `$9AD000` | (408, 880) | **P** |
| `$839243` O | `$889A6A` → `$889A6F` | `$83ED67` | (376, 912) | **X** |
| `$83924D` O | `$88A1A4` → `$88A1A9` | `$83EDF8` | (424, 928) | **X** |
| `$839257` O | `$888038` → `$88803D` | `$83ED4F` | (8, 16) | **X** |
| `$839261` FE | `$88805D` → `$888062` | `$9AD000` | (8, 0) | **E** |
| `$839266` FF | `$858008` | — | — | **T** |

### Map $21, list $839268

| Record / type | Header → entry | Descriptor / implicit base | Source position | Disposition |
|---|---|---|---|---|
| `$83926A` FD | `$84A129` → `$84A12E` | `$9AD000` | (136, 128) | **P** |
| `$839271` O | `$88B2FA` → `$88B2FF` | `$83F88E` | (120, 272) | **H** |
| `$83927B` O | `$88AEB8` → `$88AEBD` | `$83F8C0` | (136, 384) | **X** |
| `$839285` FE | `$88AE5F` → `$88AE64` | `$9AD000` | (8, 0) | **X** |
| `$83928F` O | `$88ACF5` → `$88ACFA` | `$83F984` | (136, 384) | **V** |
| `$83929E` FE | `$88805D` → `$888062` | `$9AD000` | (8, 0) | **X** |
| `$8392A3` O | `$888038` → `$88803D` | `$83ED4F` | (8, 16) | **X** |
| `$8392AD` FE | `$88AD84` → `$88AD89` | `$9AD000` | (8, 0) | **H** |
| `$8392B2` FF | `$858008` | — | — | **T** |

### Native membership, branches and descendants

- **B:** resident `$888E4B` retains with `$0074` clear and `$0027 XOR $0021`
  false. Hidden interactions retain with `$0025` clear plus their additional
  interaction gates. Entry dialogue `$888E66..8E78` uses text `$888FDA`;
  callback `$888EDE` uses `$888FF0` and sets `$0026` at `$888F08`.
- **C:** all four residents reject `$002A XOR $002C` true. With `$002F` clear,
  shared headers A321/9A6A/A1A4 retain only in C (`COP49 $800C`); their copies in
  E/20 are therefore not additional fresh residents. Other event branches
  change visibility, palette, location and pose. Source `$88AAE9` requires
  `$0028` set and `$0292` clear: absent fresh. Controller `$88ACA9` survives
  while `$0028` is clear, watches player X160..208/Y376..392, displays text,
  sets local `$0001` at `$88ACC8`, restores input and deletes. Its activation
  explains the final itinerary's changed local flag, not a cellar opening.
  The wooden C→B door is a **tile interaction**, not a missing NPC:
  `$87923F → $87C7F1`, tile low-nine ID `$00F3`, handler `$8797CA`.
- **D:** candidate `$888BEB` retains only when `$0023 XOR $0021` is true,
  so its stale slot is not a fresh resident. Hidden `$88A9AF` retains while
  `$0026` is clear (`COP48 $8026`) and installs occupancy (`COP3B`). Visible
  `$88A837` uses `COP26 04 0A 29 29` randomized bounded movement at `$88A868`.
  While `$0026` is clear, the warning tests player X112..128/Y696..712 and
  Down (`$88A872..A892`). It is not a teleport or a general pursuit algorithm.
- **F:** opening NPC `$889752` runs while `$0020` is clear, completes the
  introduction, sets that flag, moves and deletes. `$888AB5` requires `$0020`
  set, so it is absent on the first load but retained on return. Hidden parent
  `$88D60D` creates child `$88D641` at `$88D618` via `COP9C`, relative `(0,-16)`.
  Child setup `$88D64D` selects `$A2C000`, selector 42 at `$88D652`, loop
  `$88D655`. The parent's callback `$88D63B` invokes scene routine `$878590`.
- **10:** `$002A XOR $002C` selects alternate positions/selector 9. Otherwise
  `$0023 XOR $0107` false selects the ordinary right/left loops; the opposite
  state changes palette/behavior and transfers to `$88D32D`. Hidden `$889A46`
  registers callback `$889A54`; initialization creates no visible child.
- **11:** analogous resident branch conditions apply. The ordinary loop
  `$88A9DE` selects 4, H-flipped; local `$0001` changes continuation. Hidden
  `$88C88E` can redirect Ark and alter tiles/events. Its three compact services
  do not add three residents or three unexplained OAM objects.
- **E/20:** shared residents reject these maps under fresh conditions. Shared
  `$888038` rejects them immediately. `$88805D` creates effect helper `$888029`
  (register queue `32E7,3040,2521`) and deletes; neither is an ordinary resident.
- **21:** `$88B2FA` retains while `$0101` is clear; with `$0023` clear it hides
  and waits for `$0003`, then moves. `$88AEB8` requires `$0023` clear and `$0022`
  set; otherwise it deletes. `$88AE5F` needs `$0244` clear and `$0022` set.
  With `$0244` clear the list creates special visual `$88ACF5` (selector 3,
  player/PPU/event changes), not another standing NPC. With `$0244` set it
  instead creates the `$88805D` effect service. `$88AD84` is a timed dialogue/
  event controller while `$0022` is clear. Later `$88B3B0` creates six finite
  effects at B3B4/B3BB/B3C2/B3C9/B3D0/B3D7, entries `$88B573,B58D,B5A7,B5C1,B5DB,B5F5`.
  These are later scene effects, not six fresh residents. No 21 phase is yet
  admitted for portable rendering.

Every ordinary list first takes the fresh fallthroughs for `$01AC` and `$0196`.
B/C additionally fall through `$00BA XOR $00BB`. Source FA addresses/targets:

| Map | `$01AC` branch | `$0196` branch | Other branch |
|---|---|---|---|
| B | `838B85 → 83EA28` | `838B8A → 838BDF` | `838B8F → 838BBA` (`BA XOR BB`) |
| C | `838BF9 → 83EA34` | `838BFE → 838C72` | `838C03 → 838C4D` (`BA XOR BB`) |
| D | `838CAA → 83EA5E` | `838CAF → 838CE2` | — |
| F | `838D27 → 83EA6F` | `838D2C → 838D58` | — |
| 10 | `838D72 → 83EA85` | `838D77 → 838DAF` | `838D9F → 838DA6`, same as fallthrough; does not remove FD interaction |
| 11 | `838DD8 → 83EAA0` | `838DDD → 838E0E` | — |
| 21 | — | — | `83928A → 839299` if 244 set; `839299 → 8392A3` if clear |

These alternatives replace the ordinary list; **do not add their residents to
the fresh count**. B/C BA/BB alternatives use bank-8C scripts; 196 alternatives
primarily use `$9793E4/$9795A7`; 1AC alternatives use bank-97 actors. Their full
later-story behavior is outside this fresh branch census.

### Shared and transitive services: every settled linked entity

Ark initialization creates eight direct helpers. Three surviving helpers create
three more normal actors; two initial helpers immediately delete. Thus there
are **nine settled Ark helpers**, not eleven permanent residents:

| Creation source | Entry | Fresh disposition |
|---|---|---|
| `$84A147` | `$8480E8` | nonvisual control/movement helper |
| `$84A14E` | `$8487CE` | nonvisual action/arrival helper |
| `$84A174` | `$84BFC2` | hidden action/item sprite actor; fresh idle `$84C01F` |
| `$84A17B` | `$8DB907` | immediate deletion: map table `$8DBA2A` begins `$FFFF` |
| `$84A182` | `$8DB848` | one-shot object-state initializer; then deletes at `$8DB904` |
| `$84A189` | `$84BDD4` | hidden auxiliary animation actor; fresh idle `$84BDF5` |
| `$84A190` | `$84A8FF` | **visible shadow**, follows Ark, `$A2C000` selector 2 |
| `$84A197` | `$879084` | nonvisual camera controller |
| `$87908A` | `$87919F` | nonvisual player-state projection helper |
| `$84BFC2` | `$85EF04` | auxiliary UI/output service; no fresh output in selected captures |
| `$84BFC9` | `$84C199` | nonvisual player/item synchronization |

`$8DB848` skips its optional `$0123` decompression for every house map, optionally
initializes three five-byte object records via `$96DDBD`, and deletes without
creating children or drawing. `$8DB907` does not become another helper. Slot reuse
must not be mistaken for a surviving actor or initialization data.

The repeated room record `$888038`, descriptor `$83ED4F`, is not a resident. It
immediately rejects E/1F/20/21; elsewhere it requires `$002A XOR $002C` true.
When retained in later states it creates `$888000`, queuing scene registers
`2C16,2D00,3020,31A3`. It is deleted on the selected fresh paths. Nonzero unlinked
slot bytes are explicitly reported as **remnants, not membership**.

Every FF appends `$858008`; its normal path also creates compact helper `$8580D3`
via `COPAA` at `$858014`. FB records and these compact allocations are separate
from the normal `$0DFC` entity list. They are accounted as services, not inferred
residents. Exact OAM coverage at all selected visited checkpoints finds **no
unexplained compact-actor output**. This does not mean these services can never
render or alter backgrounds in other states.

### D scene label: source-backed auxiliary OAM, not a world prop

The list terminator `$838CD9` is followed by the encoded four-glyph string at
`$838CDA..8CE2`. Loader `$80F4AC..F4E9` stores its pointer in `$7F0806` and appends
`$858008`. That controller parses the text at `$85869B`; glyph sources are
`$B5A880`, `$B69580`, `$B494C0`, `$B591C0` (64 bytes each, not decoded in this task).
`$858656..869A` centers four glyphs with 12-pixel advance: X104/116/128/140, Y48.
`$858024` installs attribute `$3400`: palette 2, **priority 3**.

The resulting six-byte records occupy `$7F0992`, extent `$7F0810=24`. Source
`$80ECEA` emits this text buffer **before** the ordinary entity pass, without
camera subtraction, setting large-size bits. Actual OBJSEL=2 gives 16×16.
The checker correlates every glyph's queue record, OAM low/high bytes and source
text pointer; arbitrary leftover OAM is rejected, not relabeled “overlay.” At
the later D checkpoint the queue is empty and all four glyphs are gone. Exact
lifetime scheduling/font transformation remains separate work; do not bake this
label into a permanent actor raster.

Other missing full-scene surfaces include the already documented secondary
background/window/sunlight effects, door tile mutations, dialogue and action/UI
output. The source/node census separates these from missing NPC assets rather
than claiming first-background-plus-actors is a complete native framebuffer.

## Reproduce the first bounded result

```sh
python3 -B tools/house-scene-qualification/test_census.py
python3 -O -B tools/house-scene-qualification/test_census.py
sh tools/house-scene-qualification/replay.sh 'local/Tenchi Souzou (Japan).sfc'
```

`probe.rs` authenticates the ROM, starts `Session::new` with empty SRAM, reuses
`tools/new-game-qualification/bootstrap.rs` through completed 6800, then consumes
an explicit finite real-button itinerary. No RAM patches, warps, restored states
or `save_state` calls occur. Captures use passive WRAM/VRAM/CGRAM/pixels and
`Session::sprite_state` access. A second itinerary tests F's exceptional exit;
it is another fresh boot, not a restoration. Replay runs **each itinerary twice
in separate processes**.

The deliberately descriptive exploration labels are not trusted room IDs:
`settledB` was still C before the wooden door interaction; `exteriorA`/`outside`
are still gated D. `check.py` verifies actual map IDs at selected checkpoints.
The main itinerary enters F→10→C→B→C→D→11→D, probes the blocked exterior, returns
to C and probes the cellar approach. It never claims an E/20/21/A/122 visit.

`census.py` extracts all linked members plus clearly separated unlinked remnants,
correlates each drawn composition with exact hardware OAM and size/ninth-X bits,
checks all ordinary depth/tie order and qualifies the only auxiliary remainder
against D's text queue. Every active OAM slot in the selected captures is owned:
residents, Ark, shadow, F's object, or the temporary D label. The result is **not
an all-family source-art comparison**; new graphics/palette decoding follows in
a separately reviewed task. Source frame/resource pointers, branch conditions,
script-created children and nonvisual dispositions come from the ROM research
above, not a numeric runtime initialization snapshot.

`roster.py` is a bounded RE projection of the nine selected source lists. It does
not execute conditions or scripts and is not a production spawn VM. The golden
reference contains source-derived metadata and hashes only. Full census JSON,
ROM bytes, source text dumps, tiles, palettes, OAM, WRAM and screenshots stay in
ignored `local/`. `check.py --record` is an explicit maintainer reference-update
operation, never part of replay.

Synthetic tests were red before exit/list traversal, then green for boundaries,
truncation, cycles and linked-vs-stale membership. OAM correlation was initially
experimental RE with retrospective regression tests. Independent review found
unowned-overlay and vacuous ordering-test gaps: new tests were red before the
bounded overlay verifier and extracted depth verifier, then green for two-actor
unequal/equal depths, reversed list order, special-path exclusions and unexplained
or tampered text output. Checks use explicit exceptions and survive Python `-O`.

### Repeatability boundary discovered during final replay

The initial two itinerary executions matched all selected surfaces. A later
four-boot replay at `local/house-scene-qualification/replay-AD147t` reproduced
all selected **WRAM, VRAM, CGRAM, OAM and OBJSEL/first-sprite bytes**, source
membership, poses and ordering, but the `cellar-blocked` framebuffer differed in
15,365 RGBA pixels within output X16..495/Y199..231 (the dialogue band). This is
not hidden by selecting another pose or modifying captures. `pixels()` is kept
as a local diagnostic, not an atomic census surface or a whole-frame equality
claim. The paired/golden census excludes **only its framebuffer hash**, and a
synthetic regression proves that OAM/other hardware hash changes still affect
the census fingerprint. Full local `census.py` output retains pixel hashes for
inspection. New actor art-to-pixel qualification remains a separate next task.

Final end-to-end replay succeeded at
`local/house-scene-qualification/replay-gVc1Gx`, including normal and optimized
Python checks. Eight synthetic tests pass. Independent final correctness/
architecture review found no blockers. The cause of the earlier dialogue-band
pixel discrepancy is **not diagnosed** by matching endpoint hardware buffers;
root-cause investigation is separate from this census qualification.

## Decoder phase: complete admitted frozen setup roster

This phase completes the **source-art** work deferred by the census above. It
admits exactly nine residents plus F's child `$88:D618`, not the shadow or any
additional controller/effect. `assets::sprites::HouseScenes::from_rom(image)`
projects the caller-authenticated Japanese ROM into immutable `HouseActor`s.
It is deliberately **frozen fresh setup**, not an event VM, animation clock,
collision implementation, dialogue system or wandering simulation.

### Public consumer contract

```rust,ignore
use assets::sprites::{HouseScenes, SpritePixel};

let scenes = HouseScenes::from_rom(rom.image())?;
for actor in scenes.actors().iter().filter(|a| a.map_id() == map_id) {
    let frame = actor.setup_frame(); // explicitly record zero
    let composition = frame.composition();
    let bounds = composition.bounds(actor.hflip(), false);
    // Local signed coordinates relative to actor.position(); no extra Y+1.
    match composition.sample(actor.graphics(), actor.hflip(), false, x, y)? {
        SpritePixel::Transparent => {}
        SpritePixel::Opaque { palette_index, priority, .. } => {
            let color = actor.palette()[usize::from(palette_index - actor.palette_base())];
            // Retain priority and alpha; color.rgb8() is natural, not emulator gamma.
        }
    }
}
```

- `actors()` is the complete roster; `actor(source_id)` is exact lookup with no
  fallback. Instance identity is the source spawn record, or F's child-creation
  instruction, **not a runtime WRAM slot, frame key or shared raster key**.
- `position()`, `selector()`, `hflip()`, `facing()` and `map_id()` are source
  setup metadata. Facing uses native Down=0, Up=1, Left=2, Right=3. Vertical
  mirroring is not requested. Native OAM name-select `$100` is **not** added to
  the source-indexed graphics resource. Shared `SpriteFrame` already implements
  the hardware Y convention; the consumer must not add another pixel.
- `frames()` retains every bounded list record in source order, including
  repeated keys and duration bytes. `setup_frame()` always selects record zero.
  Duration 7 is not a promise to advance every seven host frames: native list
  scheduling/AI is not implemented. C and 11 expose their full selected lists;
  D also exposes its four-record **creation** list, not later AI selectors.
- `HousePoseKey::Compressed { packet, offset }` keeps the CPU packet address
  separate from its decoded composition offset. `Direct(address)` is a real ROM
  composition address. Both point four bytes after the corresponding anchor.
  `HouseGraphicsKey::Compressed(address)` identifies the 256-tile graphics
  packet. Composition packets and graphics are cached; instances may share art.
- `source_composition()` preserves unrelocated source bytes. `composition()`
  applies native palette relocation while preserving component order, offsets,
  flips, priority and source tile IDs. `palette_base()` is 160, 192 or 208;
  `palette()` retains the sixteen source BGR555 colors, with color zero
  transparent. Only used colors are needed for actor rendering.
- `source_ranges()` on both actor and roster returns exact normalized headerless
  input extents, including reuse dependencies and consumed packet terminators.
  Ranges may repeat/overlap. Descriptor zero reuses the previous relocated
  resource; graphics `$FFFF` reuses **only graphics**, not the new palette/list.
  Neither reuse form can borrow a predecessor from another room.
- Ordinary back-to-front sorting is ascending `(world_y, tie_rank())`.
  `HouseScenes::ark_tie_rank(map)` gives Ark's last rank at equal Y. It returns
  `None` outside the six admitted rooms. Shadow/text use other ordering classes.
  Existing `HouseNpc` methods, const accessors, source-range order and first-NPC
  raster output remain compatible through a thin shared-loader adapter; that
  legacy load does not require the other nine actors to exist.

| Source ID | Map | Source origin | Selector | Setup facing / H-flip | Palette base | Tie rank |
|---|---|---|---|---|---|---|
| `$838B96` | B | `(120,112)` | 6 | Down / no | 208 | 0 |
| `$838C0A` | C | `(88,416)` | 4 | Up / no | 208 | 3 |
| `$838C14` | C | `(56,384)` | 5 | Right / no | 208 | 2 |
| `$838C1E` | C | `(72,368)` | 3 | Down / no | 192 | 1 |
| `$838C28` | C | `(104,368)` | 3 | Down / no | 192 | 0 |
| `$838CB4` | D | `(72,672)` | 3 | **Down / no** | 192 | 0 |
| `$838D7C` | 10 | `(424,416)` | 2 | Right / no | 208 | 1 |
| `$838D86` | 10 | `(440,416)` | 2 | Left / yes | 192 | 0 |
| `$838DE2` | 11 | `(440,640)` | 4 | Up / yes | 192 | 0 |
| `$88D618` | F | `(472,144)` | `$42` | Right / no | 160 | 0 |

Ark's ranks are B=1, C=4, D=1, F=1, 10=2, 11=1. F's retained facing byte is
metadata for its direct frame, not a claim that the table object turns.

### D creation is not its later arrival screenshot

A fresh trace follows the ordinary source loader, stopping on the entity whose
source cursor has just advanced from `$838CB4` to `$838CBE` and whose first script
is `$88:A83C`. The paired probes discover entity `$1040`; at completed creation
`$80:F5E4`, frame 8099, it has origin `(72,672)`, selector 3, facing Down, no
H-flip, composition `$7E:7163`, anchors from that composition, timer zero and
flags `$5100/$0000/$0100`. Earlier stops at `$80:F5D3`, `$80:ED99`, `$80:EE11`
and the later stop **before** `$88:A83C` bracket initialization and first AI use.

The initialized record-zero composition is **not emitted to OAM before the
scheduler/AI runs** on this fresh route. Its creation origin/pose is qualified
against initialized WRAM and source metadata, not a pre-AI framebuffer. Its
complete `$D8:1022` list and graphics/palette are identical to C's `$838C28`,
whose distinct list poses have separate native OAM/VRAM/CGRAM witnesses. A later
D capture facing Up or at `(104,672)` must not replace this setup contract.

### Art qualification and explicit framebuffer limits

```sh
cargo test -p assets --lib --test sprites --test local_sprites
cargo clippy -p assets --all-targets -- -D warnings
python3 -B tools/house-scene-qualification/test_art.py
python3 -O -B tools/house-scene-qualification/test_art.py
sh tools/house-scene-qualification/art-replay.sh 'local/Tenchi Souzou (Japan).sfc'
```

`export.rs` uses only the ROM and public decoder; it exports all ten actors and
all 28 list records (repeated keys retained). `art-route.jsonl` has exactly the
same frame-by-frame real inputs as the census route; it only splits passive
waits into additional capture boundaries. `art-replay.sh` runs two independent
empty-SRAM native processes and two independent fresh D creation probes. No
warps, patches, restored states or `save_state` calls are used. Raw ROM, art,
WRAM, OAM and pixels remain ignored under `local/`. The committed
`art-reference.json` contains metadata/hashes only; `art_check.py --record` is a
separate explicit maintainer operation, never part of replay.

The independent checker validates source ranges/hashes, palette relocation,
all bounded compositions in native relocated WRAM, direct F ROM composition,
component anchors, OAM coordinates/attributes/order/size/ninth-X bits and
ordinary source tie ranks including Ark. All 256 resident source tiles match
native VRAM. F's used tiles `$AD..AF` match its separate scene graphics upload;
Ark dynamically overwrites unrelated low tiles, so the complete F resource is
not compared wholesale to VRAM. A separate planar decoder and compositor
reproduce every exported indexed, natural-RGBA and priority raster. All distinct
non-D list frames have actual hardware OAM witnesses; repeated records retain
separate source positions but can share the same art witness. D is qualified as
described above, not counted as another pre-AI OAM witness.

Two qualifications are intentionally narrower than full-frame equality:

1. Native `$8D:AC09` conditionally writes white `$7FFF` to `$7F:079C`, uploaded by
   `$85:F98F` from `$7F:0600` to CGRAM. This is index 206 (base192/color14), whose
   source value is `$7C1F`. The checker explicitly validates that override and
   that **no admitted resident list frame uses it**. Every used color matches
   the source palette. The API retains the source palette rather than baking
   scene-global palette effects into resident assets.
2. Isolated opaque framebuffer pixels match using the emulator's documented
   gamma/output layout. B, 10, 11 and C's upper/side-facing residents match all
   opaque pixels. C's two seated Down-facing residents use the fixed local
   `y < -8` mask, excluding their table-occluded lower bodies. F uses fixed local
   `x < 3`; its right-side final BG/effect composition is **not qualified** here.
   Its whole OBJ raster still matches source, OAM, used VRAM tiles and palette.
   Complete opaque-pixel match counts, including excluded regions, remain in
   the reference as diagnostics. These masks are fixed, not inferred by
   accepting whichever pixels happened to match. Sample checkpoints are after
   pose changes settle: endpoint OAM changes precede the captured framebuffer
   by two samples here. No full-frame equality or complete BG/effect rendering
   claim follows from this work.

TDD covered the new roster/prop API and pure art helpers. Independent production
review found and prompted red/green fixes for base-relative bank crossings and
compressed selectors entering packet payload instead of the selector table.
Malformed/reuse/relocation tests cover exact roster, D creation policy, distinct
reuse palettes, inherited provenance, record metadata, component boundaries and
prop script shapes. The admitted resources do not use large components crossing
tile column 15 or the last resource row; this loader rejects those cases rather
than silently using the generic compositor's unqualified boundary behavior.
Native trace discovery and capture selection were experimental RE; their
regression checks were added after discovery, not claimed as prospective TDD.

End-to-end paired art replay passed normal and optimized checks at
`local/house-scene-qualification/art-tHjpM3`. The remaining work is consumer
integration and separately scoped native behavior/BG/effects, not another
missing admitted resident family. Optional Ark shadow, D's transient OBJ3 text,
E/20/21 phase visuals and all controllers remain excluded from this decoder.

Independent qualification review found no blockers. Six pure art tests pass in
normal and optimized Python, as does the complete asset test suite. A freshly
compiled legacy export (`legacy-export-v4`) remains byte-identical to the
original first-NPC metadata, tiles, both compositions and all three raster
products; the earlier census golden also remains unchanged.

## Spawn record format

Verified against all nine documented origins in the table above. Each actor
spawn is a ten-byte record opening with `$01`:

| Byte | Meaning |
| ---: | --- |
| 0 | record opcode; `$01` for the nine below, but `$00` and `$FD` records carry positions too |
| 1 | tile X |
| 2 | tile Y |
| 3 | usually `$00`; `$83:8C32` carries `$80` here |
| 4..7 | script pointer |
| 7..10 | second pointer |

The origin is **`(tile_x * 16 + 8, tile_y * 16)`**, not `tile * 16`. That half-cell
horizontal bias is what makes the decode checkable: all nine records reproduce
their documented origins exactly. Brute-forcing `origin = (s*rec[i]+bx,
s'*rec[j]+by)` over byte indices, scales and offsets yields exactly one fit, and
the scale is pinned by differences (tile 26 to 27 is 16 pixels) rather than by
residues alone.

The encoding is not limited to `$01` records: `$00` and `$FD` records use it as
well, and far more than nine rows in the inventory above fit it.

```text
$83:8B96  01 07 07 00 ...  -> ( 7,  7) -> (120, 112)
$83:8C0A  01 05 1a 00 ...  -> ( 5, 26) -> ( 88, 416)
$83:8C14  01 03 18 00 ...  -> ( 3, 24) -> ( 56, 384)
$83:8C1E  01 04 17 00 ...  -> ( 4, 23) -> ( 72, 368)
$83:8C28  01 06 17 00 ...  -> ( 6, 23) -> (104, 368)
$83:8CB4  01 04 2a 00 ...  -> ( 4, 42) -> ( 72, 672)
$83:8D7C  01 1a 1a 00 ...  -> (26, 26) -> (424, 416)
$83:8D86  01 1b 1a 00 ...  -> (27, 26) -> (440, 416)
$83:8DE2  01 1b 28 00 ...  -> (27, 40) -> (440, 640)
```

### The per-map list

The loader at `$80:F3FD` is `LDX $0480` / `LDA $828000,X` / `LDA $838000,X`, so
the table base is **`$83:8000`**, indexed by map ID times two. Every map in the
Crysta slice resolves to a list, and all nine documented records above are
reached from their map's list:

```text
map $000B -> list $83:8B7C   (contains $8B96)
map $000C -> list $83:8BF0   (contains $8C0A, $8C14, $8C1E, $8C28)
map $000D -> list $83:8CA1   (contains $8CB4)
map $0010 -> list $83:8D69   (contains $8D7C, $8D86)
map $0011 -> list $83:8DCF   (contains $8DE2)
```

`$83:8D69` is the same address this document's own table already names at the
`$82:8020 = 0` row, which is the cross-check that the base is right.

### Stream grammar

The list ends at `$FF`; `$80:F4A4` terminates the walk there, so bytes after
the terminator are `$FA` branch targets rather than fall-through. Element
lengths come from the interpreter at `$80:F4EA` and its handlers:

| Opcode | Length | Source |
| --- | ---: | --- |
| `$00`, `$01` | 10, or **16** when byte 3's top two bits are both set | `$80:F564` reads four further fields |
| `$FD` | 7 | `$80:F5F9` |
| `$FB`, `$FE` | 5 | |
| `$FF` | 2 | terminator, `$80:F4A4` |
| `$FA` | 5, or **7** when the condition word is non-negative with bits in `$F800` | `$80:F759`, `$80:F76C` |

`$FA` is a chained event-flag condition, evaluated through `$80:BBC7` — the
same routine the map loading scripts use. `$80:F76C` branches on bit 15 *before*
masking `$F800`, so a negative word takes the short form. Getting that order
wrong misparses exactly one map, `$0021`.

All 24 Crysta lists decode end to end under this grammar, 115 records total.
`assets::maps::actors::SpawnList` implements it and refuses an opcode outside
the set rather than resynchronising.

### Conditions

`$FA <condition> <target>` is a conditional branch, not a marker. `$80:F760`
tests the flag through `$80:BBC7` — the same routine the loading scripts use,
so the index is `word & $0FFF` — and then:

- a non-negative word reaches `$80:F7DB` and **branches when the flag is set**;
- a negative word reaches `$80:F7E1` and **branches when it is clear**.

Note this is the opposite convention to the loading script's `$08 FD`.

A chained condition accumulates: `$80:F787` adds each word's result with `ADC`
and `$80:F765` re-normalises with `AND #$0001`, so a chain is the **parity** of
its results, with the sense taken from the last word read. The `$4000` and
`$2000` variants at `$80:F773` and `$80:F778` take other paths and are not
decoded; `SpawnList::resolve` refuses them.

Following those branches against the measured new-game flag state resolves all
24 Crysta lists. Every resident the census documents for the six visited rooms
is present, and the `$01` records reproduce its counts in four of them. The two
that exceed it each contain a documented hidden actor — map `$000D`'s excess is
exactly the `(120,720)` occupancy actor named above — which is what identifies
the difference as the census's own exclusion rather than a decode error.

### What is not decoded

The `$4000`/`$2000` condition variants. The scripts each record points at are
not executed, so this says which actors are installed, not what they do.

The installer at `$80:F52B` stores `tile * 16` with no bias, yet the running
game reports the origin eight pixels right. That `+8` is applied after
installation and its source is not decoded; the origins above are what the game
shows, cross-checked against actor positions read out of WRAM while walking the
reference emulator.
