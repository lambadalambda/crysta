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
    ArkSprites, HouseActor, HouseFrame, Mode4Art, PandoraArt, PandoraSprites, RecordRefusal,
    ResidentPose, SpriteError, SpriteFrame, SpritePixel,
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
    /// Row-major `0xAARRGGBB`; alpha is zero (transparent), `0xFF`, or
    /// `0xFE` for OBJ palettes 4 to 7, the ones colour math takes
    /// ([`MATH_ALPHA`]).
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

/// The first colour of OBJ palette 4: palettes 4 to 7 take colour math,
/// 0 to 3 never do.
const MATH_PALETTES: u8 = 192;
/// A raster pixel's alpha in OBJ palettes 4 to 7.
pub const MATH_ALPHA: u8 = 0xFE;

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
    palette: &[Bgr555],
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
                    let alpha = if palette_index >= MATH_PALETTES {
                        MATH_ALPHA
                    } else {
                        0xFF
                    };
                    u32::from(alpha) << 24 | u32::from(r) << 16 | u32::from(g) << 8 | u32::from(b)
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
    /// The same frames, every opaque pixel marked as colour math takes it
    /// or not ([`MATH_ALPHA`]).
    #[must_use]
    pub fn with_math(mut self, math: bool) -> Self {
        let alpha = u32::from(if math { MATH_ALPHA } else { 0xFF }) << 24;
        for pixel in self
            .frames
            .iter_mut()
            .flat_map(|frame| frame.pixels.iter_mut())
        {
            if *pixel >> 24 != 0 {
                *pixel = (*pixel & 0x00FF_FFFF) | alpha;
            }
        }
        self
    }

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
    /// Ark's thrust lists with the spear ([`Mode4Art::thrust`]); empty when
    /// they do not decode.
    thrust: Vec<Mode4Art>,
    /// Ark's lists that hold the spear on the rope: resource 1's `$0F` and
    /// `$10` ([`Mode4Art::with_weapon`]).
    rope: Vec<(u8, Mode4Art)>,
}

/// Ark's resource 1, whose rope lists hold the spear.
const ROPE_ART: u32 = 0x80_A255;
const ROPE_LISTS: [u8; 2] = [0x0F, 0x10];

/// The id [`CarryArt::animation`] knows Ark's thrust by: resource 4's base.
pub const THRUST: u32 = 0xA5_A000;
/// The spear, whose colours the thrust takes.
const SPEAR: u8 = 0x81;

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
            thrust: (0..3)
                .map(|list| Mode4Art::thrust(image, SPEAR, list))
                .collect::<Result<_, _>>()
                .unwrap_or_default(),
            rope: ROPE_LISTS
                .into_iter()
                .filter_map(|list| Some((list, Mode4Art::with_weapon(image, 1, SPEAR, list).ok()?)))
                .collect(),
        })
    }

    /// An art's list as rasters, as
    /// [`PandoraSprites::carry_pose`] names them.
    ///
    /// # Errors
    /// Refuses an art or a list the decoder does not hold, or frames outside
    /// the qualified shape.
    pub fn animation(&self, art: u32, selector: u8, hflip: bool) -> Result<Animation, ArtError> {
        // The lists that hold the spear; the rope art's others are the
        // stairs', below.
        let held = match art {
            THRUST => Some(
                self.thrust
                    .get(usize::from(selector))
                    .ok_or(SpriteError::Invalid("no such thrust list"))?,
            ),
            ROPE_ART => self
                .rope
                .iter()
                .find(|(list, _)| *list == selector)
                .map(|(_, art)| art),
            _ => None,
        };
        if let Some(held) = held {
            return animate(
                held.list().frames(),
                (held.graphics(), held.palette(), held.palette_base()),
                hflip,
            );
        }
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
#[derive(Debug, Clone)]
pub struct Body {
    art: BodyArt,
}

#[derive(Debug, Clone)]
enum BodyArt {
    House(HouseActor),
    /// One list of a Pandora art (Pandora's Box, an object sheet's list),
    /// rasterized unmirrored and mirrored.
    List(u8, [Animation; 2]),
    /// A mode-4 descriptor's list (`docs/mode4-descriptors.md`), and the
    /// descriptor, whose other lists are decoded as they are shown.
    Mode4(usize, u8, [Animation; 2]),
}

/// The OBJ palette the frozen townsfolk show (`docs/scene-effects.md`).
pub const FROZEN_SLOT: u8 = 3;

/// OBJ palette 3 after the freeze: `$CC:2A6C` (European `$CE:2A6C`), with
/// colour 14 patched to `$08DF` (its source is not found).
#[must_use]
pub fn frozen_palette(image: &[u8]) -> Option<[Bgr555; 16]> {
    let at = assets::layout::per_revision(image, 0x0C_2A6C, 0x0E_2A6C);
    let bytes = image.get(at..at + 32)?;
    let mut palette: [Bgr555; 16] = std::array::from_fn(|index| {
        Bgr555::new(u16::from_le_bytes([bytes[index * 2], bytes[index * 2 + 1]]))
    });
    palette[14] = Bgr555::new(0x08DF);
    Some(palette)
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
            BodyArt::List(list, animations) | BodyArt::Mode4(_, list, animations)
                if selector == *list =>
            {
                Ok(animations[usize::from(hflip)].clone())
            }
            BodyArt::List(..) | BodyArt::Mode4(..) => {
                Err(SpriteError::Invalid("no such list").into())
            }
        }
    }

    /// The OBJ palette slot the body shows under a `COP BB` field: its own
    /// plus the field, modulo 8. `None` for a list body, which keeps its own.
    #[must_use]
    pub const fn shown_slot(&self, field: u8) -> Option<u8> {
        match &self.art {
            BodyArt::House(actor) => Some((actor.palette_base() / 16 + field) & 7),
            BodyArt::List(..) | BodyArt::Mode4(..) => None,
        }
    }

    /// [`Self::animation`] in another palette's colours.
    ///
    /// # Errors
    /// As [`Self::animation`].
    pub fn recoloured(
        &self,
        selector: u8,
        hflip: bool,
        palette: &[Bgr555],
    ) -> Result<Animation, ArtError> {
        match &self.art {
            BodyArt::House(actor) => animate(
                &actor.sequence(selector, hflip)?,
                (actor.graphics(), palette, actor.palette_base()),
                hflip,
            ),
            BodyArt::List(..) | BodyArt::Mode4(..) => self.animation(selector, hflip),
        }
    }

    /// [`Self::animation`] as a `COP BB` field shows it: in the frozen
    /// palette when the field moves the body to [`FROZEN_SLOT`], in its own
    /// otherwise (other shifted slots are not known).
    ///
    /// # Errors
    /// As [`Self::animation`].
    pub fn shown(
        &self,
        image: &[u8],
        (selector, hflip): (u8, bool),
        field: u8,
    ) -> Result<Animation, ArtError> {
        let frozen = (field != 0 && self.shown_slot(field) == Some(FROZEN_SLOT))
            .then(|| frozen_palette(image))
            .flatten();
        let shown = match (&self.art, frozen) {
            // Another list of the descriptor: Shadowkeeper's shots and
            // wisps are its own spawns' poses.
            (BodyArt::Mode4(descriptor, list, _), _) if selector != *list => {
                Self::mode4(image, *descriptor, selector)?.animation(selector, hflip)?
            }
            (_, Some(palette)) => self.recoloured(selector, hflip, &palette)?,
            (_, None) => self.animation(selector, hflip)?,
        };
        // Colour math follows the slot shown, not the one decoded.
        Ok(match self.shown_slot(field) {
            Some(slot) => shown.with_math(slot >= 4),
            None => shown,
        })
    }

    /// The header's initial selector.
    #[must_use]
    pub const fn initial(&self) -> u8 {
        match &self.art {
            BodyArt::House(actor) => actor.initial(),
            BodyArt::List(list, _) | BodyArt::Mode4(_, list, _) => *list,
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

    fn mode4(image: &[u8], descriptor: usize, selector: u8) -> Result<Self, ArtError> {
        let art = Mode4Art::from_rom(image, descriptor, selector)?;
        let animate = |hflip| {
            animate(
                art.list().frames(),
                (art.graphics(), art.palette(), art.palette_base()),
                hflip,
            )
        };
        Ok(Self {
            art: BodyArt::Mode4(descriptor, selector, [animate(false)?, animate(true)?]),
        })
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

/// A resident whose descriptor (its own, or the one it reuses) is mode
/// `$0004` and not the Box's: its header's list (byte 0, five bytes before
/// the script) of that art (`docs/mode4-descriptors.md`), as tower 1's
/// statues and plaque.
fn mode4_body(image: &[u8], resident: &Resident) -> Option<Result<Body, Placeholder>> {
    let descriptor = resident.descriptor?;
    // Mode `$04` with the plain palette forms (`$40`, or 0 as the hooded
    // guardians, whose byte 4 names their profile); the table forms are the
    // house decoder's.
    if *image.get(descriptor + 3)? != 4
        || ![0x00, 0x40].contains(image.get(descriptor + 8)?)
        || assets::layout::offset(image, BOX_DESCRIPTOR) == Some(descriptor)
    {
        return None;
    }
    // A spawned child has no header of its own: its pose is its parent's
    // list, its lists the parent's descriptor's.
    let list = if resident.record == 0 {
        resident.selector
    } else {
        let header = usize::try_from(resident.script? & 0x3F_FFFF)
            .ok()?
            .checked_sub(5)?;
        *image.get(header)?
    };
    // A header byte past the packet's lists (the light room's orb, `$1F`)
    // is not a list: the pose the record starts in is.
    Some(
        Body::mode4(image, descriptor, list)
            .or_else(|_| Body::mode4(image, descriptor, resident.initial))
            .map_err(|error| Placeholder::Refused(error.to_string())),
    )
}

/// A list of the helper art (`$A2:C000`) as rasters: an enemy's explosion,
/// a dropped gem.
///
/// # Errors
/// Refuses a list outside the qualified shapes.
pub fn overlay_animation(image: &[u8], base: u32, selector: u8) -> Result<Animation, ArtError> {
    list_animation(&PandoraArt::object(image, base, selector)?, selector, false)
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
    (graphics, palette, base): (&[Tile4bpp], &[Bgr555], u8),
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
    let mut bodies: Vec<_> = present
        .iter()
        .map(|resident| {
            let found = list
                .iter()
                .position(|record| record.offset() == resident.record)
                .and_then(|index| decoded[index].take());
            if let Some(own) = own_art(image, resident) {
                // The object sheet is qualified in the tour rooms, for the
                // desk's book in the bedroom and the freeze's crystals in
                // `$21` only; elsewhere what the list draws is not known,
                // and the blue door's target in C shows nothing natively.
                // The helper art is the same everywhere for the children
                // scripts spawn (bullets) and in the towers (the
                // Magirocks); the blue door's target in C is not drawn.
                let helper = own.0 == crate::actors::helper(image)
                    && (resident.record == 0 || crate::TOWER_MAPS.contains(&map));
                return if (0x41..=0x44).contains(&map) || [0x0F, 0x21].contains(&map) || helper {
                    Body::object(image, own)
                        .map_err(|error| Placeholder::Refused(error.to_string()))
                } else {
                    Err(Placeholder::Invisible)
                };
            }
            if let Some(body) = mode4_body(image, resident) {
                return body;
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
        .collect();
    // A child a script spawned draws as its parent (`$80:BCA4` copies the
    // art with `+$00..$17`): the Cadets' spells. Only in the towers, where
    // it is checked.
    for (index, resident) in present.iter().enumerate() {
        if resident.record != 0
            || !crate::TOWER_MAPS.contains(&map)
            || !matches!(bodies[index], Err(Placeholder::Invisible))
        {
            continue;
        }
        let parent = present.iter().zip(&bodies).find_map(|(other, body)| {
            (other.record != 0
                && other.descriptor.is_some()
                && other.descriptor == resident.descriptor)
                .then(|| body.as_ref().ok())
                .flatten()
        });
        if let Some(body) = parent.cloned() {
            bodies[index] = Ok(body);
        }
    }
    bodies
}

/// The object sheet and list a resident's script points its art at before
/// its first pose: `COP D8 base`, then `COP 80 list`, with `COP B2`
/// (an offset) and `COP 48` (a flag check) allowed first, as the spear's
/// display `$89:D9FE` and the blue door's hit target `$88:AAEE` do, and a
/// bullet's set-up (`$97:BA9C`: `COP D9`, its own words, `COP 82`) and
/// the Magirock's taken test (`$84:DD83`). Its
/// descriptor, often reused from the record before, is not what it draws.
fn own_art(image: &[u8], resident: &Resident) -> Option<(u32, u8)> {
    let mut at = usize::try_from(resident.script? & 0x3F_FFFF).ok()?;
    let mut base = None;
    let mut back = None;
    for _ in 0..12 {
        // The Magirock's taken test (`$84:DD83`): on to its art when not.
        if let Some(&[0xBD, 0x26, 0, 0x29, 0xFF, 0, 0x18, 0x69, _, _, 0x22, _, _, _, 0x90, skip]) =
            image.get(at..at + 16)
        {
            at += 16 + usize::from(skip);
            continue;
        }
        let code = image.get(at..at + 9)?;
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
            // `COP 80 pose`, `81`/`82` with movement, or `COP D0`'s orbit
            // with its pose word.
            [2, 0x80..=0x82 | 0xD0] => return base.map(|base| (base, code[2])),
            // `LDA #0; STA $0004,X` or `$0006,X`: a cleared entity, as the
            // desk's book starts (`$88:D641`).
            [0xA9, 0] if code[2..4] == [0, 0x9D] && matches!(code[4..6], [4 | 6, 0]) => at += 6,
            // `LDA $0006,X; ORA #$4000; STA $0006,X`, as the crystals do.
            [0xBD, 6] if code[2..] == [0, 0x09, 0, 0x40, 0x9D, 6, 0] => at += 9,
            // `COP D9`'s profile; `LDA #n` into the entity's own words:
            // `STA $7F:xxxx,X`, the direction `STA`/`STZ $0014,X`.
            [2, 0xD9] | [0xA9, _] => at += 3,
            [0x9F, _] if code[3] == 0x7F => at += 4,
            [0x9D | 0x9E, 0x14] if code[2] == 0 => at += 3,
            // `JSR` within the bank and its `RTS`, one level deep.
            [0x20, low] if back.is_none() => {
                back = Some(at + 3);
                at = (at & 0xFF_0000) | usize::from(u16::from_le_bytes([low, code[2]]));
            }
            [0x60, _] => at = back.take()?,
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
