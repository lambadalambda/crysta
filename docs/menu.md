# The menu: Yomi's box

Research for [menus, inventory and saves](../meta/issues/menus-inventory-save.md),
from native captures on both ROMs.

Select opens the menu (`COP 2D 0020 → $85:A179`, European `$85:A211`, in the
player controllers `$84:88F0…`), only once event flag `$FE` is set (the
frozen return): before that Select does nothing. Start is a separate pause
(`$80:820C`). The menu is Yomi's box: a Mode 7 room with hotspots, Yomi the
cursor; the field's actors stop (`$049A`); Select leaves (about 60 frames in,
40 out).

| Hotspot | Screen |
|---|---|
| item door | 8×3 grid of the 27 shared slots, a row of fixed slots; A asks Use / Equip (`$0648`, the field's X uses it); L shows the description |
| weapon, armor doors | 12 pedestals each; A equips (`$064A`, `$064C`) |
| mirror | status: attack, defense (+bonus), luck, Prime Blue, EXP, next |
| memo | options: keys (`$0634–063E`), cursor memory (`$06B4` bit 14), sound, window colour, text speed, gauge |
| jewel box | magic (a separate screen) |
| map, shelves | map; move and battle guides |

Data: level `$0656`, max life `$0657`, life `$065D`, attack `$0662`, defense
`$065F`, luck `$0666`, weapon/armor bonus `$0659`/`$065B`, EXP `$0690` (BCD,
3 bytes). Equipment stats: `$8D:BC92` (European `$8D:BB5B`), 4 bytes per item
from `$80` (power, attribute). Art: the room `$AB:8ABA` (as the tour's), the
HUD sheet `$A9:F02F` (the shop's), item icons as the shop's.

In the slice the shop items (`$1E`: S.Bulb, M.Bulb, P.Cure, Hex Rod, Leather;
`$1D`: Fire Ring, Ice Ring) and the Crystal Spear can end up in the menu.
