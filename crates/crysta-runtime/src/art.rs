//! Decoded sprite art for the slice: the player's frames and each resident's
//! setup frame, rasterized once into pixels a renderer can blit.
//!
//! Everything here is derived from the ROM through [`assets::sprites`]. The
//! rasters carry their own placement relative to the actor's origin, and a
//! mirrored raster is composed here from the ROM's alternate anchors rather
//! than flipped again by a renderer.

use crate::residents::Resident;
use assets::graphics::{Bgr555, Tile4bpp};
use assets::maps::actors::SpawnList;
use assets::maps::scripts::EventFlags;
use assets::sprites::{
    ArkSprites, HouseActor, HouseFrame, PandoraArt, PandoraSprites, RecordRefusal, ResidentPose,
    SpriteError, SpriteFrame, SpritePixel,
};
use room_core::{AnimationFrame, AnimationSet};
use std::fmt;

/// The OBJ priority ordinary sprites carry; anything else is unqualified.
const ORDINARY_PRIORITY: u8 = 2;

/// One sprite frame as pixels, placed relative to the actor's world origin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Raster {
    /// Width in pixels.
    pub width: usize,
    /// Height in pixels.
    pub height: usize,
    /// Offset of the top-left pixel from the actor's origin.
    pub offset: (i16, i16),
    /// Row-major `0xAARRGGBB`; alpha is `0xFF` or zero, nothing between.
    pub pixels: Vec<u32>,
}

impl Raster {
    /// Whether any pixel is opaque.
    #[must_use]
    pub fn is_visible(&self) -> bool {
        self.pixels.iter().any(|pixel| pixel >> 24 != 0)
    }
}

/// A frame the renderer cannot use.
#[derive(Debug)]
pub enum ArtError {
    /// A component carries a priority other than the ordinary one.
    Priority {
        /// The priority found.
        priority: u8,
    },
    /// A pixel names a palette entry outside the actor's sixteen.
    Palette {
        /// The CGRAM index found.
        index: u8,
    },
    /// The sprite decoder refused the frame.
    Sprite(SpriteError),
}

impl fmt::Display for ArtError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Priority { priority } => write!(f, "sprite priority {priority} is not ordinary"),
            Self::Palette { index } => write!(f, "palette index {index} is outside the actor's"),
            Self::Sprite(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for ArtError {}

impl From<SpriteError> for ArtError {
    fn from(error: SpriteError) -> Self {
        Self::Sprite(error)
    }
}

/// Rasterizes one composition into placed pixels.
///
/// Mirroring selects the ROM's alternate placements and anchors as well as
/// flipping tiles, which is why it is done here and not by the renderer.
///
/// # Errors
/// Refuses a component outside the ordinary priority, a pixel outside the
/// palette, and a tile the graphics do not hold.
pub fn raster(
    frame: &SpriteFrame,
    tiles: &[Tile4bpp],
    palette: &[Bgr555; 16],
    palette_base: u8,
    mirror: bool,
) -> Result<Raster, ArtError> {
    let (left, top, right, bottom) = frame.bounds(mirror, false);
    // A composition without components folds to inverted bounds.
    let (Some(width), Some(height)) = (
        right
            .checked_sub(left)
            .and_then(|w| usize::try_from(w).ok()),
        bottom
            .checked_sub(top)
            .and_then(|h| usize::try_from(h).ok()),
    ) else {
        return Err(ArtError::Sprite(SpriteError::Invalid(
            "empty sprite composition",
        )));
    };
    let mut pixels = Vec::with_capacity(width * height);
    for y in top..bottom {
        for x in left..right {
            pixels.push(match frame.sample(tiles, mirror, false, x, y)? {
                SpritePixel::Transparent => 0,
                SpritePixel::Opaque {
                    palette_index,
                    priority,
                    ..
                } => {
                    if priority != ORDINARY_PRIORITY {
                        return Err(ArtError::Priority { priority });
                    }
                    let colour = palette_index
                        .checked_sub(palette_base)
                        .and_then(|index| palette.get(usize::from(index)))
                        .ok_or(ArtError::Palette {
                            index: palette_index,
                        })?;
                    let [r, g, b] = colour.rgb8();
                    0xFF00_0000 | u32::from(r) << 16 | u32::from(g) << 8 | u32::from(b)
                }
            });
        }
    }
    Ok(Raster {
        width,
        height,
        offset: (left, top),
        pixels,
    })
}

/// The player's twenty-eight ordinary frames: three standing, eighteen
/// walking, and the horizontal ones again mirrored for facing left.
#[derive(Debug, Clone)]
pub struct ArkAtlas {
    frames: Vec<(AnimationFrame, Raster)>,
}

impl ArkAtlas {
    /// Rasterizes every ordinary frame from the ROM.
    ///
    /// # Errors
    /// Propagates a sprite table the decoder refuses.
    pub fn from_rom(image: &[u8]) -> Result<Self, ArtError> {
        let sprites = ArkSprites::from_rom(image)?;
        let mut frames = Vec::new();
        for set in [AnimationSet::Standing, AnimationSet::Walking] {
            let records = if set == AnimationSet::Standing { 1 } else { 6 };
            for sequence in 0..3u8 {
                for record in 0..records {
                    for mirror_x in [false, true] {
                        // Only the horizontal sequence has a mirrored twin.
                        if mirror_x && sequence != 2 {
                            continue;
                        }
                        let key = AnimationFrame {
                            set,
                            sequence,
                            record,
                            mirror_x,
                        };
                        let source = &sprites.frames()[frame_index(key)];
                        let tiles = sprites
                            .graphics(source.resource())
                            .ok_or(SpriteError::Invalid("missing ordinary graphics"))?;
                        let pixels = raster(
                            source.composition(),
                            tiles,
                            sprites.palette(),
                            128,
                            mirror_x,
                        )?;
                        frames.push((key, pixels));
                    }
                }
            }
        }
        Ok(Self { frames })
    }

    /// The raster for an animation key.
    ///
    /// # Panics
    /// Every key [`room_core::AnimationState`] can produce is present, so a
    /// miss is a programming error rather than a runtime condition.
    #[must_use]
    pub fn frame(&self, key: AnimationFrame) -> &Raster {
        self.frames
            .iter()
            .find(|(candidate, _)| *candidate == key)
            .map(|(_, raster)| raster)
            .expect("every ordinary animation key is rasterized")
    }

    /// Number of frames held.
    #[must_use]
    pub fn len(&self) -> usize {
        self.frames.len()
    }

    /// Whether the atlas is empty, which it never is once built.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }
}

/// Position of an animation key in the ROM's frame list order: standing
/// down/up/horizontal, then walking down/up/horizontal at six each.
fn frame_index(frame: AnimationFrame) -> usize {
    match frame.set {
        AnimationSet::Standing => usize::from(frame.sequence),
        AnimationSet::Walking => 3 + usize::from(frame.sequence) * 6 + usize::from(frame.record),
    }
}

/// A resident's pose list as rasters, cycled by each record's duration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Animation {
    /// One raster per record, in list order.
    pub frames: Vec<Raster>,
    /// Raw native countdown bytes: each record lasts `duration + 1` ticks.
    pub durations: Vec<u8>,
}

impl Animation {
    /// The raster showing `tick` frames after the list started.
    ///
    /// The list loops: the ordinary loop re-selects the pose and waits for
    /// it to resolve, so a resident cycles their records for as long as they
    /// stand there. $80:EDA0 stores the raw duration at actor+$0E, and
    /// $80:C72C decrements before testing negative: zero therefore lasts one
    /// tick. The resident's script restarts a list with `COP 80` and waits for
    /// it with `COP 8E`; the loop here stands in only where it does not.
    #[must_use]
    pub fn frame_at(&self, tick: u64) -> &Raster {
        let total: u64 = self.durations.iter().map(|d| u64::from(*d) + 1).sum();
        let mut remaining = if total == 0 { 0 } else { tick % total };
        for (raster, duration) in self.frames.iter().zip(&self.durations) {
            let hold = u64::from(*duration) + 1;
            if remaining < hold {
                return raster;
            }
            remaining -= hold;
        }
        &self.frames[0]
    }

    /// The raster `tick` frames in when the list plays once: after its end,
    /// its last frame holds.
    #[must_use]
    pub fn frame_once(&self, tick: u64) -> &Raster {
        let mut remaining = tick;
        for (raster, duration) in self.frames.iter().zip(&self.durations) {
            let hold = u64::from(*duration) + 1;
            if remaining < hold {
                return raster;
            }
            remaining -= hold;
        }
        self.frames.last().unwrap_or(&self.frames[0])
    }
}

/// Ark's carry poses and the pots' sprites ([`PandoraSprites`]), as
/// animations on demand.
#[derive(Debug)]
pub struct CarryArt {
    sprites: PandoraSprites,
    /// Ark's brake and dash lists ([`PandoraSprites::run_art`]).
    run: [PandoraArt; 2],
}

impl CarryArt {
    /// A pot's list in flight, after the throw's release (`$84:C701`).
    pub const FLIGHT: u8 = 0x3C;

    /// Decodes the art from the ROM.
    ///
    /// # Errors
    /// Propagates a sprite table the decoder refuses.
    pub fn from_rom(image: &[u8]) -> Result<Self, ArtError> {
        Ok(Self {
            sprites: PandoraSprites::from_rom(image)?,
            run: PandoraSprites::run_art(image)?,
        })
    }

    /// An art's list as rasters, as
    /// [`PandoraSprites::carry_pose`] names them.
    ///
    /// # Errors
    /// Refuses an art or a list the decoder does not hold, or frames outside
    /// the qualified shape.
    pub fn animation(&self, art: u32, selector: u8, hflip: bool) -> Result<Animation, ArtError> {
        let art = self
            .sprites
            .art()
            .iter()
            .chain(&self.run)
            .find(|candidate| candidate.source_id() == art && candidate.list(selector).is_some())
            .ok_or(SpriteError::Invalid("no such carry art or list"))?;
        list_animation(art, selector, hflip)
    }
}

/// Why a resident has no art.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Placeholder {
    /// The record installs no graphics resource, so the game draws nothing
    /// for it either: a `$FD` record is a script with a position,
    /// not a body. Nothing should be drawn.
    Invisible,
    /// The record's descriptor, pose or frame was refused. Something stands
    /// here in the game; what it looks like is not established.
    Refused(String),
    /// The record reuses a predecessor's resource, and that predecessor was
    /// itself refused, so reusing it would draw the wrong body.
    PredecessorRefused,
}

/// A resident's decoded body: every sequence of their packet on demand.
#[derive(Debug)]
pub struct Body {
    art: BodyArt,
}

#[derive(Debug)]
enum BodyArt {
    House(HouseActor),
    /// One list of a Pandora art (Pandora's Box, an object sheet's list),
    /// rasterized unmirrored and mirrored.
    List(u8, [Animation; 2]),
}

/// Pandora's Box in `$21` (`$83:928F`): descriptor `$83:F984`, mode
/// `$0004`, which the ordinary loader does not take; [`PandoraSprites`]
/// decodes it with its one list, selector 3, under the Japanese address in
/// either revision (European `$83:F92C`).
const BOX_DESCRIPTOR: usize = 0x03_F984;
const BOX_ART: u32 = 0x83_F984;
const BOX_SELECTOR: u8 = 3;

impl Body {
    /// The animation for a sequence and mirror, as a running script selects
    /// them: standing 0 to 2, walking 3 to 5.
    ///
    /// # Errors
    /// Refuses a selector the packet does not hold, or frames outside the
    /// qualified shape.
    pub fn animation(&self, selector: u8, hflip: bool) -> Result<Animation, ArtError> {
        match &self.art {
            BodyArt::House(actor) => animate(
                &actor.sequence(selector, hflip)?,
                (actor.graphics(), actor.palette(), actor.palette_base()),
                hflip,
            ),
            BodyArt::List(list, animations) if selector == *list => {
                Ok(animations[usize::from(hflip)].clone())
            }
            BodyArt::List(..) => Err(SpriteError::Invalid("no such list").into()),
        }
    }

    /// The header's initial selector.
    #[must_use]
    pub const fn initial(&self) -> u8 {
        match &self.art {
            BodyArt::House(actor) => actor.initial(),
            BodyArt::List(list, _) => *list,
        }
    }

    fn pandora_box(image: &[u8]) -> Result<Self, ArtError> {
        let sprites = PandoraSprites::from_rom(image)?;
        let art = sprites
            .get(BOX_ART)
            .ok_or(SpriteError::Invalid("no box art"))?;
        Self::list(art, BOX_SELECTOR)
    }

    /// The list `selector` of the object sheet at `base`.
    fn object(image: &[u8], (base, selector): (u32, u8)) -> Result<Self, ArtError> {
        Self::list(&PandoraArt::object(image, base, selector)?, selector)
    }

    fn list(art: &PandoraArt, selector: u8) -> Result<Self, ArtError> {
        Ok(Self {
            art: BodyArt::List(
                selector,
                [
                    list_animation(art, selector, false)?,
                    list_animation(art, selector, true)?,
                ],
            ),
        })
    }
}

/// A Pandora art's list as rasters.
fn list_animation(art: &PandoraArt, selector: u8, hflip: bool) -> Result<Animation, ArtError> {
    let list = art
        .list(selector)
        .ok_or(SpriteError::Invalid("no such Pandora list"))?;
    animate(
        list.frames(),
        (art.graphics(), art.palette(), art.palette_base()),
        hflip,
    )
}

/// Frames as rasters from their graphics, palette and palette base.
fn animate(
    frames: &[HouseFrame],
    (graphics, palette, base): (&[Tile4bpp], &[Bgr555; 16], u8),
    hflip: bool,
) -> Result<Animation, ArtError> {
    Ok(Animation {
        frames: frames
            .iter()
            .map(|frame| raster(frame.composition(), graphics, palette, base, hflip))
            .collect::<Result<_, _>>()?,
        durations: frames.iter().map(HouseFrame::duration).collect(),
    })
}

/// Bodies for each resident of a map, aligned with `present`.
///
/// Records are decoded in the order the stream executed them under the
/// flags at map entry, `spawned`, because a record may reuse the resource or
/// graphics of the record parsed before it; the loader owns that chain and
/// refuses a reuse whose predecessor it refused. Poses follow `events`, the
/// flags now, which a conversation may have changed since.
#[must_use]
pub fn residents_art(
    image: &[u8],
    map: u16,
    present: &[Resident],
    spawned: EventFlags<'_>,
    events: EventFlags<'_>,
) -> Vec<Result<Body, Placeholder>> {
    let Ok(list) = SpawnList::resolve(image, map, spawned) else {
        return present
            .iter()
            .map(|_| Err(Placeholder::Refused("spawn list refused".into())))
            .collect();
    };
    let mut decoded: Vec<Option<Result<HouseActor, RecordRefusal>>> =
        HouseActor::from_records(image, map, &list, |record| {
            ResidentPose::from_script(image, record, events).unwrap_or_default()
        })
        .into_iter()
        .map(Some)
        .collect();
    present
        .iter()
        .map(|resident| {
            let found = list
                .iter()
                .position(|record| record.offset() == resident.record)
                .and_then(|index| decoded[index].take());
            if let Some(own) = own_art(image, resident) {
                // The object sheet is qualified in the tour rooms only;
                // elsewhere what the list draws is not known, and the blue
                // door's target in C shows nothing natively.
                return if (0x41..=0x44).contains(&map) {
                    Body::object(image, own)
                        .map_err(|error| Placeholder::Refused(error.to_string()))
                } else {
                    Err(Placeholder::Invisible)
                };
            }
            match found {
                None | Some(Err(RecordRefusal::NoDescriptor)) => Err(Placeholder::Invisible),
                Some(Err(RecordRefusal::PredecessorRefused)) => {
                    Err(Placeholder::PredecessorRefused)
                }
                Some(Err(RecordRefusal::Invalid(_)))
                    if assets::layout::offset(image, BOX_DESCRIPTOR)
                        .is_some_and(|box_| resident.descriptor == Some(box_)) =>
                {
                    Body::pandora_box(image)
                        .map_err(|error| Placeholder::Refused(error.to_string()))
                }
                Some(Err(RecordRefusal::Invalid(error))) => {
                    Err(Placeholder::Refused(error.to_string()))
                }
                Some(Ok(actor)) => Ok(Body {
                    art: BodyArt::House(actor),
                }),
            }
        })
        .collect()
}

/// The object sheet and list a resident's script points its art at before
/// its first pose: `COP D8 base`, then `COP 80 list`, with `COP B2`
/// (an offset) and `COP 48` (a flag check) allowed first, as the spear's
/// display `$89:D9FE` and the blue door's hit target `$88:AAEE` do. Its
/// descriptor, often reused from the record before, is not what it draws.
fn own_art(image: &[u8], resident: &Resident) -> Option<(u32, u8)> {
    let mut at = usize::try_from(resident.script? & 0x3F_FFFF).ok()?;
    let mut base = None;
    for _ in 0..8 {
        let code = image.get(at..at + 5)?;
        match code[..2] {
            [2, 0x48 | 0xB2] => at += 4,
            [2, 0xD8] => {
                base = Some(
                    0x80_0000
                        | u32::from(code[4]) << 16
                        | u32::from(code[3]) << 8
                        | u32::from(code[2]),
                );
                at += 5;
            }
            [2, 0x80] => return base.map(|base| (base, code[2])),
            _ => return None,
        }
    }
    None
}

#[cfg(test)]
mod timing_tests {
    use super::{Animation, Raster};

    fn animation(durations: &[u8]) -> Animation {
        Animation {
            frames: durations
                .iter()
                .enumerate()
                .map(|(i, _)| Raster {
                    width: 1,
                    height: 1,
                    offset: (0, 0),
                    pixels: vec![u32::try_from(i).unwrap()],
                })
                .collect(),
            durations: durations.to_vec(),
        }
    }

    #[test]
    fn native_countdown_seven_holds_eight_ticks_and_four_records_take_32() {
        let animation = animation(&[7, 7, 7, 7]);
        for tick in 0..96 {
            assert_eq!(
                u64::from(animation.frame_at(tick).pixels[0]),
                (tick / 8) % 4
            );
        }
    }

    #[test]
    fn zero_is_one_tick_and_maximum_byte_is_256_ticks() {
        let animation = animation(&[0, 255]);
        assert_eq!(animation.frame_at(0).pixels[0], 0);
        assert_eq!(animation.frame_at(1).pixels[0], 1);
        assert_eq!(animation.frame_at(255).pixels[0], 1);
        assert_eq!(animation.frame_at(256).pixels[0], 1);
        assert_eq!(animation.frame_at(257).pixels[0], 0);
        assert_eq!(
            animation.frame_at(u64::MAX),
            animation.frame_at(u64::MAX % 257)
        );
    }
}
