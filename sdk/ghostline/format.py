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

    def __init__(
        self,
        f,
        started_at: int,
        git_sha: bytes | None = None,
        parent_run_id: bytes | None = None,
        fork_at_step: int | None = None,
    ):
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

        # Fork metadata
        if parent_run_id and fork_at_step is not None:
            f.write(b"\x01")
            f.write(parent_run_id)  # 32 bytes
            f.write(struct.pack("<I", fork_at_step))
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

        # Read fork metadata
        has_fork = f.read(1)
        if has_fork == b"\x01":
            self.parent_run_id = f.read(32)
            (self.fork_at_step,) = struct.unpack("<I", f.read(4))
        else:
            self.parent_run_id = None
            self.fork_at_step = None

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


def fork(source_path: str, at_step: int, output_path: str | None = None) -> str:
    """Fork a .ghostline file at a specific step.

    Copies frames 0..=at_step into a new file with parent lineage metadata.

    Args:
        source_path: Path to the source .ghostline file.
        at_step: Step index (inclusive) to fork at.
        output_path: Destination path. Defaults to <source>-fork-<step>.ghostline.

    Returns:
        Path to the new forked file.
    """
    if output_path is None:
        stem = source_path.removesuffix(".ghostline")
        output_path = f"{stem}-fork-{at_step}.ghostline"

    with open(source_path, "rb") as f:
        reader = GhostlineReader(f)

        if at_step >= reader.frame_count:
            raise IndexError(
                f"step {at_step} out of range — file has {reader.frame_count} frames"
            )

        # Compute parent_run_id: SHA-256(started_at || first_frame_hash)
        first_frame = reader.get_frame(0)
        parent_run_id = hashlib.sha256(
            struct.pack("<Q", reader.started_at) + first_frame.request_hash
        ).digest()

        frames = [reader.get_frame(i) for i in range(at_step + 1)]

    with open(output_path, "wb") as out:
        writer = GhostlineWriter(
            out,
            started_at=reader.started_at,
            git_sha=reader.git_sha,
            parent_run_id=parent_run_id,
            fork_at_step=at_step,
        )
        for frame in frames:
            writer.append(frame)
        writer.finish()

    return output_path
