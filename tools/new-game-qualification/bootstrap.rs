//! Shared input-only path through the real Japanese menu and opening dialogue.
use oracle::Button;

pub const INPUTS: &[(Button, u32, u32)] = &[
    (Button::Start, 400, 410),
    (Button::Down, 700, 712),
    (Button::Down, 730, 742),
    (Button::Down, 760, 772),
    (Button::A, 900, 912),
    (Button::Start, 1200, 1210),
    (Button::A, 3500, 3512),
    (Button::A, 4100, 4112),
    (Button::A, 4700, 4712),
    (Button::A, 5300, 5312),
    (Button::A, 5900, 5912),
];
