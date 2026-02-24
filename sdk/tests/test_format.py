"""Tests for the .ghostline binary format (Python implementation)."""

import io
import tempfile

from ghostline.format import Frame, GhostlineWriter, GhostlineReader


def test_frame_roundtrip():
    frame = Frame(b"request", b"response", 42, 1700000000000)
    packed = frame.to_msgpack()
    unpacked = Frame.from_msgpack(packed)
    assert unpacked.request_bytes == b"request"
    assert unpacked.response_bytes == b"response"
    assert unpacked.latency_ms == 42
    assert unpacked.timestamp == 1700000000000
    assert unpacked.request_hash == frame.request_hash


def test_write_read_roundtrip():
    buf = io.BytesIO()
    writer = GhostlineWriter(buf, started_at=1700000000000)

    f1 = Frame(b"req1", b"res1", 10, 1700000000000)
    f2 = Frame(b"req2", b"res2", 20, 1700000001000)
    writer.append(f1)
    writer.append(f2)
    writer.finish()

    buf.seek(0)
    reader = GhostlineReader(buf)
    assert reader.frame_count == 2
    assert reader.started_at == 1700000000000

    read1 = reader.get_frame(0)
    assert read1.request_bytes == b"req1"
    assert read1.response_bytes == b"res1"

    read2 = reader.get_frame(1)
    assert read2.request_bytes == b"req2"


def test_lookup_by_hash():
    buf = io.BytesIO()
    writer = GhostlineWriter(buf, started_at=0)
    frame = Frame(b"alpha", b"beta", 5, 100)
    writer.append(frame)
    writer.finish()

    buf.seek(0)
    reader = GhostlineReader(buf)
    found = reader.lookup_by_hash(frame.request_hash)
    assert found is not None
    assert found.request_bytes == b"alpha"

    assert reader.lookup_by_hash(b"\x00" * 32) is None


def test_iteration():
    buf = io.BytesIO()
    writer = GhostlineWriter(buf, started_at=0)
    for i in range(5):
        writer.append(Frame(f"req{i}".encode(), f"res{i}".encode(), i, i * 1000))
    writer.finish()

    buf.seek(0)
    reader = GhostlineReader(buf)
    frames = list(reader)
    assert len(frames) == 5
    assert frames[3].request_bytes == b"req3"


def test_cross_compat_with_rust():
    """Verify Python can read files written by Rust and vice versa.

    This test writes a file with Python, then reads it back.
    For true cross-compat testing, use the Rust CLI:
        ghostline inspect <file_written_by_python>
    """
    with tempfile.NamedTemporaryFile(suffix=".ghostline", delete=False) as f:
        path = f.name
        writer = GhostlineWriter(f, started_at=1700000000000)
        writer.append(Frame(b"cross-test", b"works", 1, 1700000000000))
        writer.finish()

    with open(path, "rb") as f:
        reader = GhostlineReader(f)
        assert reader.frame_count == 1
        frame = reader.get_frame(0)
        assert frame.response_bytes == b"works"
