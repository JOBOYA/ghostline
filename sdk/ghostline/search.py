"""Semantic search across .ghostline replay frames.

Uses zvec (alibaba/zvec) when available for production-grade vector search.
Falls back to numpy cosine similarity for environments where zvec isn't installed.

Embedding strategy: TF-IDF-like bag of tokens from request/response text content.
For richer embeddings, pass a custom `embed_fn` that returns float vectors.
"""

import hashlib
import json
import os
from typing import Callable, Sequence

import numpy as np

from ghostline.format import GhostlineReader, Frame

# Type alias for embedding function: text → vector
EmbedFn = Callable[[str], np.ndarray]

# Default embedding dimension (bag-of-hashes)
_DEFAULT_DIM = 256


def _default_embed(text: str) -> np.ndarray:
    """Simple hash-based embedding for zero-dependency search.

    Projects each token into a fixed-dimension vector using feature hashing.
    Not as good as a real embedding model, but works offline with zero setup.
    """
    vec = np.zeros(_DEFAULT_DIM, dtype=np.float32)
    tokens = text.lower().split()
    for token in tokens:
        h = int(hashlib.md5(token.encode()).hexdigest(), 16)
        idx = h % _DEFAULT_DIM
        sign = 1.0 if (h >> 128) % 2 == 0 else -1.0
        vec[idx] += sign
    # L2 normalize
    norm = np.linalg.norm(vec)
    if norm > 0:
        vec /= norm
    return vec


def _frame_to_text(frame: Frame) -> str:
    """Extract searchable text from a frame."""
    parts = []
    for data in (frame.request_bytes, frame.response_bytes):
        try:
            text = data.decode("utf-8")
            parts.append(text)
        except (UnicodeDecodeError, AttributeError):
            try:
                import msgpack
                obj = msgpack.unpackb(data, raw=False)
                parts.append(json.dumps(obj, default=str))
            except Exception:
                pass
    return " ".join(parts)


class GhostlineIndex:
    """Searchable index over frames in one or more .ghostline files.

    Attributes:
        entries: List of (file_path, frame_index, frame, text) tuples.
    """

    def __init__(self, embed_fn: EmbedFn | None = None):
        self._embed_fn = embed_fn or _default_embed
        self._texts: list[str] = []
        self._vectors: list[np.ndarray] = []
        self._meta: list[tuple[str, int]] = []  # (file_path, frame_idx)
        self._zvec_collection = None

    def add_file(self, path: str) -> int:
        """Index all frames from a .ghostline file. Returns number of frames added."""
        with open(path, "rb") as f:
            reader = GhostlineReader(f)
            count = 0
            for i in range(reader.frame_count):
                frame = reader.get_frame(i)
                text = _frame_to_text(frame)
                vec = self._embed_fn(text)
                self._texts.append(text)
                self._vectors.append(vec)
                self._meta.append((path, i))
                count += 1
        self._build_index()
        return count

    def _build_index(self):
        """Build or rebuild the search index."""
        if not self._vectors:
            return

        # Try zvec first
        try:
            import zvec
            dim = len(self._vectors[0])
            self._zvec_collection = zvec.Collection(
                name="ghostline_search",
                dimension=dim,
                metric="cosine",
            )
            for i, vec in enumerate(self._vectors):
                self._zvec_collection.add(
                    id=str(i),
                    vector=vec.tolist(),
                    metadata={"file": self._meta[i][0], "frame": self._meta[i][1]},
                )
        except ImportError:
            # Fallback to numpy — stack vectors for batch cosine similarity
            self._matrix = np.stack(self._vectors)

    def search(self, query: str, k: int = 5) -> list[dict]:
        """Search for frames matching a natural language query.

        Returns list of dicts with keys: file, frame_idx, score, text_preview.
        """
        if not self._vectors:
            return []

        q_vec = self._embed_fn(query)

        # Try zvec
        if self._zvec_collection is not None:
            try:
                results = self._zvec_collection.search(
                    vector=q_vec.tolist(),
                    top_k=k,
                )
                return [
                    {
                        "file": self._meta[int(r.id)][0],
                        "frame_idx": self._meta[int(r.id)][1],
                        "score": float(r.score),
                        "text_preview": self._texts[int(r.id)][:200],
                    }
                    for r in results
                ]
            except Exception:
                pass

        # Numpy fallback: cosine similarity
        scores = self._matrix @ q_vec
        top_k = min(k, len(scores))
        if top_k >= len(scores):
            indices = np.argsort(-scores)
        else:
            indices = np.argpartition(-scores, top_k)[:top_k]
            indices = indices[np.argsort(-scores[indices])]

        return [
            {
                "file": self._meta[i][0],
                "frame_idx": self._meta[i][1],
                "score": float(scores[i]),
                "text_preview": self._texts[i][:200],
            }
            for i in indices
        ]

    @property
    def frame_count(self) -> int:
        return len(self._vectors)

    @property
    def using_zvec(self) -> bool:
        return self._zvec_collection is not None
