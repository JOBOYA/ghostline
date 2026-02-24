"""Binary format reader/writer for .ghostline files.

Mirrors the Rust implementation in ghostline-core.
Layout: [Header] [zstd(msgpack(frame))...] [Index] [index_offset: u64]
"""

import hashlib
import struct
import zstandard as zstd
import msgpack

MAGIC = b"GHSTLINE"
FORMAT_VERSION = 1


class Frame:
    """A single captured request/response pair."""

    __slots__ = ("request_hash", "request_bytes", "response_bytes", "latency_ms", "timestamp")

    def __init__(
        self,
        request_bytes: bytes,
        response_bytes: bytes,
        latency_ms: int,
        timestamp: int,
        request_hash: bytes | None = None,
    ):
        self.request_bytes = request_bytes
        self.response_bytes = response_bytes
        self.latency_ms = latency_ms
        self.timestamp = timestamp
        self.request_hash = request_hash or hashlib.sha256(request_bytes).digest()

    def to_msgpack(self) -> bytes:
        return msgpack.packb({
            "request_hash": self.request_hash,
            "request_bytes": self.request_bytes,
            "response_bytes": self.response_bytes,
            "latency_ms": self.latency_ms,
            "timestamp": self.timestamp,
        })

    @classmethod
    def from_msgpack(cls, data: bytes) -> "Frame":
        d = msgpack.unpackb(data, raw=True)
        return cls(
            request_bytes=d[b"request_bytes"],
            response_bytes=d[b"response_bytes"],
            latency_ms=d[b"latency_ms"],
            timestamp=d[b"timestamp"],
            request_hash=d[b"request_hash"],
        )


class GhostlineWriter:
    """Write frames to a .ghostline file."""

    def __init__(self, f, started_at: int, git_sha: bytes | None = None):
        self._f = f
        self._index: list[tuple[bytes, int]] = []
        self._compressor = zstd.ZstdCompressor(level=3)

        # Write header
        f.write(MAGIC)
        f.write(struct.pack("<I", FORMAT_VERSION))
        f.write(struct.pack("<Q", started_at))
        if git_sha:
            f.write(b"\x01")
            f.write(git_sha)
        else:
            f.write(b"\x00")

        self._offset = f.tell()

    def append(self, frame: Frame):
        packed = frame.to_msgpack()
        compressed = self._compressor.compress(packed)

        offset = self._offset
        self._f.write(struct.pack("<I", len(compressed)))
        self._f.write(compressed)
        self._offset += 4 + len(compressed)

        self._index.append((frame.request_hash, offset))

    def finish(self):
        index_offset = self._offset

        for req_hash, offset in self._index:
            self._f.write(req_hash)  # 32 bytes
            self._f.write(struct.pack("<Q", offset))

        self._f.write(struct.pack("<I", len(self._index)))
        self._f.write(struct.pack("<Q", index_offset))
        self._f.flush()


class GhostlineReader:
    """Read frames from a .ghostline file."""

    def __init__(self, f):
        self._f = f
        self._decompressor = zstd.ZstdDecompressor()

        # Read header
        magic = f.read(8)
        if magic != MAGIC:
            raise ValueError(f"not a .ghostline file (got {magic!r})")

        (self.version,) = struct.unpack("<I", f.read(4))
        if self.version != FORMAT_VERSION:
            raise ValueError(f"unsupported version: {self.version}")

        (self.started_at,) = struct.unpack("<Q", f.read(8))

        has_sha = f.read(1)
        self.git_sha = f.read(20) if has_sha == b"\x01" else None

        # Read index from end
        f.seek(-8, 2)
        (index_offset,) = struct.unpack("<Q", f.read(8))
        f.seek(-12, 2)
        (count,) = struct.unpack("<I", f.read(4))

        f.seek(index_offset)
        self._index: list[tuple[bytes, int]] = []
        for _ in range(count):
            req_hash = f.read(32)
            (offset,) = struct.unpack("<Q", f.read(8))
            self._index.append((req_hash, offset))

    @property
    def frame_count(self) -> int:
        return len(self._index)

    def get_frame(self, idx: int) -> Frame:
        if idx < 0 or idx >= len(self._index):
            raise IndexError(f"frame index {idx} out of range")
        _, offset = self._index[idx]
        self._f.seek(offset)
        (compressed_len,) = struct.unpack("<I", self._f.read(4))
        compressed = self._f.read(compressed_len)
        decompressed = self._decompressor.decompress(compressed)
        return Frame.from_msgpack(decompressed)

    def lookup_by_hash(self, req_hash: bytes) -> Frame | None:
        for stored_hash, offset in self._index:
            if stored_hash == req_hash:
                self._f.seek(offset)
                (compressed_len,) = struct.unpack("<I", self._f.read(4))
                compressed = self._f.read(compressed_len)
                decompressed = self._decompressor.decompress(compressed)
                return Frame.from_msgpack(decompressed)
        return None

    def __iter__(self):
        for i in range(self.frame_count):
            yield self.get_frame(i)
