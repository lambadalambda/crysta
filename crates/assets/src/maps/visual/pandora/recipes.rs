//! Audited instruction windows. Map-script, controller and COP5A pointers are distinct.
use super::super::{
    resource, scripts, validate_spans, Bgr555, Load, ResourceKind, RoomSpan, StaticBackground,
    StaticLayer, VisualMapError, ROOM_AUDIO, ROOM_COMMON, ROOM_SHARED, ROOM_SUBSCRIPTS,
};
use super::{expect, Initialization};

const CELLAR: RoomSpan = RoomSpan {
    offset: 0x18_8461,
    bytes: &[
        0x40, 0, 0x60, 0x20, 0, 0, 0, 0x80, 0, 0x30, 3, 0, 0, 0, 0, 0, 0x20, 0, 0x40, 0, 1, 0, 0,
        0, 0x20, 0, 8, 0, 0x81, 0, 0, 0, 0x20, 0, 0x40, 0, 2, 0, 0, 0, 8, 0xf8, 8, 0xfc, 5, 0, 8,
        0xff, 0x10, 0, 0,
    ],
    pointers: &[4, 11, 21, 29, 37],
};
const HOUSE13: RoomSpan = RoomSpan {
    offset: 0x18_84bf,
    bytes: &[
        0x40, 0, 0x60, 0x20, 0, 0, 0, 0x10, 1, 0, 0, 0, 0x10, 2, 0, 0, 0, 8, 0xff, 3, 0,
    ],
    pointers: &[4, 9, 14],
};
const ROOTS: &[(u16, RoomSpan)] = &[
    (
        0xe,
        RoomSpan {
            offset: 0x18_8454,
            bytes: &[8, 0xfe, 0x6b, 0, 0x10, 1, 0, 0, 0, 8, 0xfe, 0x1f, 0],
            pointers: &[6],
        },
    ),
    (
        0x13,
        RoomSpan {
            offset: 0x18_84d7,
            bytes: &[8, 0xfa, 4, 0, 0],
            pointers: &[],
        },
    ),
    (
        0x20,
        RoomSpan {
            offset: 0x18_85e9,
            bytes: &[8, 0xfa, 0x1f, 0, 0x10, 1, 0, 0, 0, 0],
            pointers: &[6],
        },
    ),
    (
        0x21,
        RoomSpan {
            offset: 0x18_85f5,
            bytes: &[8, 0xfe, 0xc1, 0, 0x10, 1, 0, 0, 0, 8, 0xff, 0x1f, 0],
            pointers: &[6],
        },
    ),
    (
        0x41,
        RoomSpan {
            offset: 0x18_831c,
            bytes: &[
                0x10, 1, 0, 0, 0, 0x20, 0, 0x40, 0, 1, 0, 0, 0, 0x20, 0, 8, 0, 0x81, 0, 0, 0, 8,
                0xfc, 0x1b, 0, 0,
            ],
            pointers: &[2, 10, 18],
        },
    ),
    (
        0x42,
        RoomSpan {
            offset: 0x18_8338,
            bytes: &[0x10, 1, 0, 0, 0, 0],
            pointers: &[2],
        },
    ),
    (
        0x43,
        RoomSpan {
            offset: 0x18_8340,
            bytes: &[0x10, 1, 0, 0, 0, 0],
            pointers: &[2],
        },
    ),
    (
        0x44,
        RoomSpan {
            offset: 0x18_8348,
            bytes: &[0x10, 1, 0, 0, 0, 0],
            pointers: &[2],
        },
    ),
];
const CONTROLLER: &[u8] = &[
    0xda, 0xa2, 0, 0, 0x86, 0x66, 0xa9, 0, 0, 0x85, 0x68, 0xa2, 0, 0, 0x8e, 0x16, 0x21, 0xa9, 0, 0,
    0x22, 0xe1, 0x84, 0x86, 0xfa,
];
const PALETTES: &[(usize, u8, u8)] = &[
    (0x9d26c, 16, 112),
    (0x9d273, 24, 8),
    (0x9d27a, 40, 8),
    (0x9d281, 56, 8),
    (0x9d288, 72, 8),
    (0x9d28f, 88, 8),
    (0x9d296, 104, 8),
];
fn table(image: &[u8], at: usize, target: usize) -> Result<(), VisualMapError> {
    expect(image, at, &(target | 0x80_0000).to_le_bytes()[..3])
}
fn root(image: &[u8], id: u16) -> Result<&'static RoomSpan, VisualMapError> {
    let span = &ROOTS
        .iter()
        .find(|r| r.0 == id)
        .ok_or(VisualMapError::Unsupported("unqualified Pandora map root"))?
        .1;
    table(image, 0x6959c + usize::from(id) * 3, span.offset)?;
    validate_spans(image, &[span])?;
    Ok(span)
}
fn subscript(image: &[u8], id: usize, target: usize) -> Result<(), VisualMapError> {
    table(image, 0x6a28c + id * 3, target)
}
fn load(
    image: &[u8],
    at: usize,
    p: usize,
    len: usize,
    kind: ResourceKind,
) -> Result<Load, VisualMapError> {
    let bytes = image
        .get(at..at + len)
        .ok_or(VisualMapError::Unsupported("truncated Pandora load"))?;
    let source = scripts::unpack_pointer(bytes[p..p + 3].try_into().expect("pointer field"), 0x98)
        .map_err(VisualMapError::Script)?
        .normalized()
        .value() as usize;
    Ok((kind, source, bytes.to_vec()))
}
fn ordinary_loads(image: &[u8], id: u16) -> Result<Vec<Load>, VisualMapError> {
    let root = root(image, id)?;
    subscript(image, 0x10, 0x18_819c)?;
    validate_spans(image, &[&ROOM_SHARED])?;
    let offsets = if id == 0x13 {
        subscript(image, 4, 0x18_84bf)?;
        subscript(image, 3, 0x18_841e)?;
        for &(index, target) in ROOM_SUBSCRIPTS.iter().filter(|r| r.0 != 1) {
            subscript(image, index, (target & 0x3f_ffff) as usize)?;
        }
        let continuation = RoomSpan {
            offset: 0x18_841e,
            bytes: &ROOM_COMMON.bytes[25..],
            pointers: &[4, 14, 22, 30],
        };
        validate_spans(image, &[&HOUSE13, &continuation, &ROOM_AUDIO])?;
        [0x18_841e, 0x18_84bf, 0x18_8427, 0x18_842f, 0x18_84c6]
    } else {
        subscript(image, 0x1f, 0x18_8461)?;
        validate_spans(image, &[&CELLAR])?;
        [0x18_8468, 0x18_8461, 0x18_8471, 0x18_8479, root.offset + 4]
    };
    [
        (ResourceKind::Graphics, offsets[0], 4, 9),
        (ResourceKind::Palette, offsets[1], 4, 7),
        (ResourceKind::Metatiles, offsets[2], 5, 8),
        (ResourceKind::Metatiles, offsets[3], 5, 8),
        (ResourceKind::Layer, offsets[4], 2, 5),
        (ResourceKind::Palette, 0x18_81a5, 4, 7),
    ]
    .into_iter()
    .map(|(k, a, p, n)| load(image, a, p, n, k))
    .collect()
}
fn cpu_source(bank: u8, address: u16) -> Result<usize, VisualMapError> {
    // Canonical HiROM or its high-half $80..BF mirror only; never mask RAM into ROM.
    if bank < 0x80 || (bank < 0xc0 && address < 0x8000) {
        return Err(VisualMapError::Unsupported("non-ROM controller pointer"));
    }
    Ok(((usize::from(bank) & 63) << 16) | usize::from(address))
}
fn controller_graphics(image: &[u8]) -> Result<usize, VisualMapError> {
    expect(image, 0x9d24e, &[0, 0, 0xd0, 0, 0])?;
    let bytes = image
        .get(0x9d253..0x9d26c)
        .ok_or(VisualMapError::Unsupported("truncated controller"))?;
    for (i, &value) in CONTROLLER.iter().enumerate() {
        if ![2, 3, 7].contains(&i) {
            expect(bytes, i, &[value])?;
        }
    }
    cpu_source(bytes[7], u16::from_le_bytes([bytes[2], bytes[3]]))
}
fn cop_palette(
    image: &[u8],
    at: usize,
    destination: u8,
    count: u8,
) -> Result<super::super::VisualResource, VisualMapError> {
    expect(image, at, &[2, 0x5a])?;
    expect(image, at + 5, &[destination, count])?;
    let source = cpu_source(
        image[at + 2],
        u16::from_le_bytes([image[at + 3], image[at + 4]]),
    )?;
    resource(
        image,
        source,
        ResourceKind::Palette,
        usize::from(count) * 2,
        false,
    )
}
pub(super) fn compile(
    image: &[u8],
    id: u16,
) -> Result<(StaticBackground, Initialization), VisualMapError> {
    if id == 0xa {
        return Ok((
            StaticBackground::from_rom(image, id)?,
            Initialization::MapLoad,
        ));
    }
    if id < 0x41 {
        return Ok((
            StaticBackground::from_loads(image, &ordinary_loads(image, id)?, 0x6000)?,
            Initialization::MapLoad,
        ));
    }
    // The route enters from 21. Its shared load supplies colors 0..15, untouched by COP5A.
    let predecessor = ordinary_loads(image, 0x21)?;
    root(image, 0x41)?;
    let current = root(image, id)?;
    let layer_source = load(image, current.offset, 2, 5, ResourceKind::Layer)?.1;
    let layer = StaticLayer::from_rom(image, layer_source).map_err(VisualMapError::Layer)?;
    let graphics = resource(
        image,
        controller_graphics(image)?,
        ResourceKind::Graphics,
        0x3000,
        true,
    )?;
    let definitions = resource(
        image,
        load(image, 0x18_8321, 5, 8, ResourceKind::Metatiles)?.1,
        ResourceKind::Metatiles,
        4096,
        true,
    )?;
    let attributes = resource(
        image,
        load(image, 0x18_8329, 5, 8, ResourceKind::Metatiles)?.1,
        ResourceKind::Metatiles,
        512,
        true,
    )?;
    let shared = resource(image, predecessor[5].1, ResourceKind::Palette, 64, false)?;
    let colors: Vec<_> = PALETTES
        .iter()
        .map(|&(a, d, n)| cop_palette(image, a, d, n))
        .collect::<Result<_, _>>()?;
    expect(image, 0x9d29d, &[0x22, 0x5c, 0x92, 0x86])?;
    let mut palette = [Bgr555::new(0); 128];
    for (color, bytes) in palette.iter_mut().zip(shared.decoded().chunks_exact(2)) {
        *color = Bgr555::new(u16::from_le_bytes([bytes[0], bytes[1]]));
    }
    for (res, &(_, destination, count)) in colors.iter().zip(PALETTES) {
        for (color, bytes) in palette[usize::from(destination)..usize::from(destination + count)]
            .iter_mut()
            .zip(res.decoded().chunks_exact(2))
        {
            *color = Bgr555::new(u16::from_le_bytes([bytes[0], bytes[1]]));
        }
    }
    let mut colors = colors.into_iter();
    let mut resources = vec![
        graphics,
        colors.next().expect("base palette"),
        definitions,
        attributes,
        shared,
    ];
    resources.extend(colors);
    let background = StaticBackground::from_parts(layer, resources, palette)?;
    // Unused definitions may refer to inherited VRAM; the complete returned sheet may not.
    if background.layer().cells().iter().any(|cell| {
        background.metatiles()[usize::from(cell.tile_index())]
            .iter()
            .any(|w| usize::from(w.tile_index()) >= background.tiles().len())
    }) {
        return Err(VisualMapError::Unsupported(
            "Pandora sheet references unsupplied controller tiles",
        ));
    }
    Ok((background, Initialization::Map21ThenController41Tour))
}

#[cfg(test)]
mod fixtures;
#[cfg(test)]
pub(super) use fixtures::{test_mutations, test_setup};
