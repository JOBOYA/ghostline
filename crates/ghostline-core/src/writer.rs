use crate::frame::Frame;
use std::io::{self, Write};

/// Magic bytes identifying a .ghostline file.
pub const MAGIC: &[u8; 8] = b"GHSTLINE";

/// Current format version.
pub const FORMAT_VERSION: u32 = 1;

/// File header written at the start of every .ghostline file.
#[derive(Debug, Clone)]
pub struct Header {
    /// Unix timestamp (ms) when the recording started.
    pub started_at: u64,
    /// Optional git SHA of the recorded project.
    pub git_sha: Option<[u8; 20]>,
}

impl Header {
    pub fn write_to<W: Write>(&self, w: &mut W) -> io::Result<()> {
        w.write_all(MAGIC)?;
        w.write_all(&FORMAT_VERSION.to_le_bytes())?;
        w.write_all(&self.started_at.to_le_bytes())?;
        match &self.git_sha {
            Some(sha) => {
                w.write_all(&[1u8])?;
                w.write_all(sha)?;
            }
            None => {
                w.write_all(&[0u8])?;
            }
        }
        Ok(())
    }
}

/// Index entry pointing to a frame's offset and its request hash.
#[derive(Debug, Clone)]
struct IndexEntry {
    request_hash: [u8; 32],
    offset: u64,
}

/// Writes frames to a .ghostline file.
///
/// Binary layout:
/// ```text
/// [Header] [zstd-compressed frame 0] [frame 1] ... [frame N] [Index] [index_offset: u64]
/// ```
///
/// The index is a sequence of (request_hash: 32 bytes, offset: u64) entries,
/// followed by a u32 entry count. The last 8 bytes of the file store the
/// byte offset where the index begins, enabling O(1) seek to any frame.
pub struct GhostlineWriter<W: Write> {
    inner: W,
    index: Vec<IndexEntry>,
    bytes_written: u64,
}

impl<W: Write> GhostlineWriter<W> {
    /// Create a new writer, immediately writing the file header.
    pub fn new(mut inner: W, header: &Header) -> io::Result<Self> {
        header.write_to(&mut inner)?;
        // Header size: 8 (magic) + 4 (version) + 8 (timestamp) + 1 (has_sha) + optional 20
        let header_size = 8 + 4 + 8 + 1 + if header.git_sha.is_some() { 20 } else { 0 };
        Ok(Self {
            inner,
            index: Vec::new(),
            bytes_written: header_size as u64,
        })
    }

    /// Append a frame, compressing it with zstd.
    pub fn append(&mut self, frame: &Frame) -> io::Result<()> {
        let msgpack = frame
            .to_msgpack()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        // Compress with zstd level 3
        let compressed = zstd::bulk::compress(&msgpack, 3)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        let frame_offset = self.bytes_written;

        // Write: [compressed_len: u32] [compressed_data]
        let len = compressed.len() as u32;
        self.inner.write_all(&len.to_le_bytes())?;
        self.inner.write_all(&compressed)?;

        self.bytes_written += 4 + compressed.len() as u64;

        self.index.push(IndexEntry {
            request_hash: frame.request_hash,
            offset: frame_offset,
        });

        Ok(())
    }

    /// Flush the index and finalize the file. Must be called when done writing.
    pub fn finish(mut self) -> io::Result<W> {
        let index_offset = self.bytes_written;

        // Write index entries: [hash: 32][offset: 8] each
        for entry in &self.index {
            self.inner.write_all(&entry.request_hash)?;
            self.inner.write_all(&entry.offset.to_le_bytes())?;
        }

        // Write entry count
        let count = self.index.len() as u32;
        self.inner.write_all(&count.to_le_bytes())?;

        // Write index offset as the final 8 bytes
        self.inner.write_all(&index_offset.to_le_bytes())?;

        self.inner.flush()?;
        Ok(self.inner)
    }

    /// Number of frames written so far.
    pub fn frame_count(&self) -> usize {
        self.index.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::Frame;

    #[test]
    fn write_and_verify_structure() {
        let mut buf = Vec::new();
        let header = Header {
            started_at: 1700000000000,
            git_sha: None,
        };

        let mut writer = GhostlineWriter::new(&mut buf, &header).unwrap();

        let frame = Frame::new(
            b"req".to_vec(),
            b"res".to_vec(),
            10,
            1700000000000,
        );
        writer.append(&frame).unwrap();
        writer.append(&frame).unwrap();
        writer.finish().unwrap();

        // Verify magic bytes
        assert_eq!(&buf[..8], MAGIC);
        // Verify version
        assert_eq!(u32::from_le_bytes(buf[8..12].try_into().unwrap()), FORMAT_VERSION);

        // Verify index offset is stored in last 8 bytes
        let len = buf.len();
        let index_offset = u64::from_le_bytes(buf[len - 8..len].try_into().unwrap());
        // Verify entry count (4 bytes before index_offset)
        let entry_count = u32::from_le_bytes(buf[len - 12..len - 8].try_into().unwrap());
        assert_eq!(entry_count, 2);
        assert!(index_offset > 0 && index_offset < len as u64);
    }
}
