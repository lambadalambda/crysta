"""Bounded ordinary-input hypothesis. No collision or accelerated-motion support.

Time is submitted-input ticks from the authenticated walking checkpoint, NOT a
claim that completed-frame WRAM $096A always decrements by exactly one.
"""
from dataclasses import dataclass, replace

CARDINALS = ('Left', 'Right', 'Up', 'Down')


class Unsupported(Exception):
    pass


class Dash(Unsupported):
    pass


@dataclass(frozen=True)
class Admission:
    previous: str = ''
    last: str = ''
    remaining: int = 0

    def submit(self, direction):
        if direction not in ('', *CARDINALS):
            raise Unsupported(direction)
        remaining = max(0, self.remaining - 1)
        last = self.last
        if direction and direction != self.previous:
            if direction == last and remaining:
                raise Dash(direction)
            last, remaining = direction, 11
        return replace(self, previous=direction, last=last, remaining=remaining)
