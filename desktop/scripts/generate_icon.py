"""Generate the app icon with only Python's standard library."""

import math
import struct
import zlib
from pathlib import Path

SIZE = 512
OUT = Path(__file__).resolve().parents[1] / "src-tauri/icons"
OUT.mkdir(parents=True, exist_ok=True)


def distance_to_segment(x, y, a, b):
    dx, dy = b[0] - a[0], b[1] - a[1]
    t = max(0, min(1, ((x - a[0]) * dx + (y - a[1]) * dy) / (dx * dx + dy * dy)))
    return math.hypot(x - a[0] - t * dx, y - a[1] - t * dy)


points = [(92, 305), (165, 213), (246, 287), (332, 188), (420, 233)]
rows = []
for y in range(SIZE):
    row = bytearray([0])
    for x in range(SIZE):
        corner = min((x - 100) ** 2 + (y - 100) ** 2,
                     (x - 412) ** 2 + (y - 100) ** 2,
                     (x - 100) ** 2 + (y - 412) ** 2,
                     (x - 412) ** 2 + (y - 412) ** 2)
        inside = 100 <= x <= 412 or 100 <= y <= 412 or corner <= 100 ** 2
        if not inside:
            row.extend((0, 0, 0, 0))
        elif min(distance_to_segment(x, y, a, b) for a, b in zip(points, points[1:])) < 17:
            row.extend((255, 255, 255, 255))
        else:
            blend = y / SIZE
            row.extend((int(87 - 21 * blend), int(111 - 24 * blend), int(241 - 15 * blend), 255))
    rows.append(bytes(row))


def chunk(kind, data):
    return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))


png = (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", SIZE, SIZE, 8, 6, 0, 0, 0))
       + chunk(b"IDAT", zlib.compress(b"".join(rows), 9)) + chunk(b"IEND", b""))
(OUT / "icon.png").write_bytes(png)
ico = struct.pack("<HHH", 0, 1, 1) + struct.pack("<BBBBHHII", 0, 0, 0, 0, 1, 32, len(png), 22) + png
(OUT / "icon.ico").write_bytes(ico)
