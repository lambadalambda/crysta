# Equip at the box's weapon and armor doors

## Summary

The weapon and armor doors open rooms of 16 pedestals each
([menu](../../docs/menu.md), "The weapon and armor rooms"): the 12 shared
slots, 4 fixed ones, three doors and the trash hole. Entering sorts the
slots by power; A asks, then equips (`$064A`, `$064C`) and the stats follow
(`$8D:BC92`); A on the equipped item asks to unequip. The rooms are parts of
the hall's map (camera (0,256) and (256,0)); the doorway is a rotating
window wipe.

## Dependencies

- [Open the box's hall with Select](box-hall.md)

## Acceptance Criteria

- Equipping the Crystal Spear and Elle's cape through the doors matches
  native traces; the walkthrough tool equips through them, and the cape's
  automatic equip goes ([cape](cape-through-armor-door.md)).

## Sub-steps

1. The room's state: entry sort, the cursor's graph, equip and unequip
   with their frames, against native WRAM traces.
2. Drawing the room: the camera, the pedestal icons on layer 2, the power
   digits and arrows, the equipped marker, Yomi.
3. The window wipe in and out.
4. The walkthrough tool equips through the box.
