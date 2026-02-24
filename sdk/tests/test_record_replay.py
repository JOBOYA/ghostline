"""Tests for record/replay context managers."""

import tempfile
from pathlib import Path

from ghostline.recorder import GhostlineRecorder
from ghostline.replayer import GhostlineReplayer
from ghostline.format import GhostlineReader


def test_recorder_captures_frames():
    with tempfile.NamedTemporaryFile(suffix=".ghostline", delete=False) as f:
        path = f.name

    recorder = GhostlineRecorder(path)
    recorder.start()
    recorder.capture(b"req1", b"res1", 10)
    recorder.capture(b"req2", b"res2", 20)
    recorder.stop()

    with open(path, "rb") as f:
        reader = GhostlineReader(f)
        assert reader.frame_count == 2


def test_recorder_context_manager():
    with tempfile.NamedTemporaryFile(suffix=".ghostline", delete=False) as f:
        path = f.name

    with GhostlineRecorder(path) as rec:
        rec.capture(b"hello", b"world", 5)

    with open(path, "rb") as f:
        reader = GhostlineReader(f)
        assert reader.frame_count == 1


def test_replayer_serves_cached():
    with tempfile.NamedTemporaryFile(suffix=".ghostline", delete=False) as f:
        path = f.name

    # Write a recording
    with GhostlineRecorder(path) as rec:
        rec.capture(b"my request", b"my response", 42)

    # Replay
    replayer = GhostlineReplayer(path)
    replayer.start()

    result = replayer.lookup(b"my request")
    assert result == b"my response"
    assert replayer.hits == 1

    result2 = replayer.lookup(b"unknown")
    assert result2 is None
    assert replayer.misses == 1

    replayer.stop()


def test_replayer_context_manager():
    with tempfile.NamedTemporaryFile(suffix=".ghostline", delete=False) as f:
        path = f.name

    with GhostlineRecorder(path) as rec:
        rec.capture(b"data", b"cached", 1)

    with GhostlineReplayer(path) as rep:
        assert rep.lookup(b"data") == b"cached"
        assert rep.hits == 1
