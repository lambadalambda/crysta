"""Terranigma's LZ packets, decoded as crates/assets/src/compression.rs does."""


def decode(image, at, limit=0x10000):
    """(data, consumed) for the packet at `at`, or None when it is not one."""
    pos = at
    control = remaining = 0

    def byte():
        nonlocal pos
        if pos >= len(image):
            raise IndexError
        pos += 1
        return image[pos - 1]

    def bit():
        nonlocal control, remaining
        if remaining == 0:
            control, remaining = byte(), 8
        value = control & 0x80
        control, remaining = (control << 1) & 0xFF, remaining - 1
        return bool(value)

    try:
        if byte() != 0:
            return None
        expected = byte() | (byte() << 8)
        if not 0 < expected <= limit:
            return None
        data = bytearray([byte()])
        while True:
            if bit():
                data.append(byte())
            elif not bit():
                length = 2 + bit() * 2 + bit()
                distance = 256 - byte()
                if distance > len(data):
                    return None
                for _ in range(length):
                    data.append(data[-distance])
            else:
                word = (byte() << 8) | byte()
                distance, length = 8192 - (word >> 3), word & 7
                if length == 0:
                    extended = byte()
                    if extended == 0:
                        return (bytes(data), pos - at) if len(data) == expected else None
                    length = extended + 1
                else:
                    length += 2
                if distance > len(data):
                    return None
                for _ in range(length):
                    data.append(data[-distance])
            if len(data) > expected:
                return None
    except IndexError:
        return None


def find(image, data, near=None):
    """Offsets whose packet decodes to `data`, nearest `near` first."""
    header = bytes([0, len(data) & 0xFF, len(data) >> 8, data[0]])
    hits, at = [], image.find(header)
    while at >= 0:
        packet = decode(image, at, len(data))
        if packet and packet[0] == data:
            hits.append(at)
        at = image.find(header, at + 1)
    if near is not None:
        hits.sort(key=lambda hit: abs(hit - near))
    return hits
