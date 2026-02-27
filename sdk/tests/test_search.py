"""Tests for semantic search across .ghostline frames."""

import tempfile
import os

from ghostline.format import Frame, GhostlineWriter
from ghostline.search import GhostlineIndex


def _make_test_file(frames_data: list[tuple[str, str]]) -> str:
    """Create a temp .ghostline file with given (request, response) text pairs."""
    path = tempfile.mktemp(suffix=".ghostline")
    with open(path, "wb") as f:
        writer = GhostlineWriter(f, started_at=1700000000000)
        for i, (req, resp) in enumerate(frames_data):
            writer.append(Frame(req.encode(), resp.encode(), 10, 1700000000000 + i))
        writer.finish()
    return path


def test_index_add_file():
    path = _make_test_file([
        ("hello world", "response one"),
        ("goodbye world", "response two"),
        ("the quick brown fox", "jumps over the lazy dog"),
    ])
    try:
        idx = GhostlineIndex()
        count = idx.add_file(path)
        assert count == 3
        assert idx.frame_count == 3
    finally:
        os.unlink(path)


def test_search_returns_results():
    path = _make_test_file([
        ("create a python function to sort numbers", "def sort(nums): return sorted(nums)"),
        ("what is the weather today", "it is sunny and 25 degrees"),
        ("explain quantum computing", "quantum computing uses qubits"),
        ("write a rust program for hello world", "fn main() { println!(\"hello\"); }"),
    ])
    try:
        idx = GhostlineIndex()
        idx.add_file(path)
        results = idx.search("python programming sorting", k=2)
        assert len(results) == 2
        assert results[0]["frame_idx"] in (0, 1, 2, 3)
        assert "score" in results[0]
        assert "file" in results[0]
    finally:
        os.unlink(path)


def test_search_relevance():
    """Verify that search ranks relevant frames higher."""
    path = _make_test_file([
        ("deploy kubernetes cluster", "kubectl apply done"),
        ("machine learning neural network training", "model accuracy 95%"),
        ("database optimization query performance", "index created on users table"),
        ("neural network deep learning pytorch", "training loss decreased"),
    ])
    try:
        idx = GhostlineIndex()
        idx.add_file(path)
        results = idx.search("machine learning neural network", k=4)
        # The ML-related frames (1 and 3) should score higher than k8s (0) and db (2)
        top_frames = {r["frame_idx"] for r in results[:2]}
        assert 1 in top_frames or 3 in top_frames, f"Expected ML frames in top 2, got {top_frames}"
    finally:
        os.unlink(path)


def test_search_empty_index():
    idx = GhostlineIndex()
    results = idx.search("anything", k=5)
    assert results == []


def test_using_zvec_property():
    idx = GhostlineIndex()
    # On Python 3.13 without zvec, should be False
    assert isinstance(idx.using_zvec, bool)
