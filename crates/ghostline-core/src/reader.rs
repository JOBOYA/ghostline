use crate::frame::Frame;
use crate::writer::{FORMAT_VERSION, MAGIC};
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct IndexEntry {
    pub request_hash: [u8; 32],
    pub offset: u64,
}

pub struct GhostlineReader<R: Read + Seek> {
    inner: R,
    pub started_at: u64,
    pub version: u32,
    pub git_sha: Option<[u8; 20]>,
    index: Vec<IndexEntry>,
}

impl GhostlineReader<io::BufReader<std::fs::File>> {
    /// Open a .ghostline file from disk.
    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        let file = std::fs::File::open(path)?;
        let reader = io::BufReader::new(file);
        Self::from_reader(reader)
    }
}

impl<R: Read + Seek> GhostlineReader<R> {
    /// Create a reader from any Read+Seek source.
    pub fn from_reader(mut inner: R) -> io::Result<Self> {
        // Read magic
        let mut magic = [0u8; 8];
        inner.read_exact(&mut magic)?;
        if &magic != MAGIC {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "invalid magic"));
        }

        // Read version
        let mut buf4 = [0u8; 4];
        inner.read_exact(&mut buf4)?;
        let version = u32::from_le_bytes(buf4);
        if version != FORMAT_VERSION {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "unsupported version"));
        }

        // Read started_at
        let mut buf8 = [0u8; 8];
        inner.read_exact(&mut buf8)?;
        let started_at = u64::from_le_bytes(buf8);

        // Read git sha
        let mut has_sha = [0u8; 1];
        inner.read_exact(&mut has_sha)?;
        let git_sha = if has_sha[0] == 1 {
            let mut sha = [0u8; 20];
            inner.read_exact(&mut sha)?;
            Some(sha)
        } else {
            None
        };

        // Read index from the end
        // Last 8 bytes = index_offset
        inner.seek(SeekFrom::End(-8))?;
        inner.read_exact(&mut buf8)?;
        let index_offset = u64::from_le_bytes(buf8);

        // 4 bytes before that = count
        inner.seek(SeekFrom::End(-12))?;
        inner.read_exact(&mut buf4)?;
        let count = u32::from_le_bytes(buf4) as usize;

        // Read index entries
        inner.seek(SeekFrom::Start(index_offset))?;
        let mut index = Vec::with_capacity(count);
        for _ in 0..count {
            let mut hash = [0u8; 32];
            inner.read_exact(&mut hash)?;
            inner.read_exact(&mut buf8)?;
            let offset = u64::from_le_bytes(buf8);
            index.push(IndexEntry {
                request_hash: hash,
                offset,
            });
        }

        Ok(Self {
            inner,
            started_at,
            version,
            git_sha,
            index,
        })
    }

    pub fn frame_count(&self) -> usize {
        self.index.len()
    }

    /// Access raw index entries.
    pub fn index_entries(&self) -> &[IndexEntry] {
        &self.index
    }

    pub fn get_frame(&mut self, index: usize) -> io::Result<Frame> {
        if index >= self.index.len() {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "frame index out of bounds"));
        }
        let offset = self.index[index].offset;
        self.inner.seek(SeekFrom::Start(offset))?;

        // Read compressed length
        let mut buf4 = [0u8; 4];
        self.inner.read_exact(&mut buf4)?;
        let len = u32::from_le_bytes(buf4) as usize;

        // Read compressed data
        let mut compressed = vec![0u8; len];
        self.inner.read_exact(&mut compressed)?;

        // Decompress
        let decompressed = zstd::bulk::decompress(&compressed, 10 * 1024 * 1024)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        // Deserialize
        Frame::from_msgpack(&decompressed)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    pub fn lookup_by_hash(&mut self, hash: &[u8; 32]) -> io::Result<Option<Frame>> {
        for i in 0..self.index.len() {
            if &self.index[i].request_hash == hash {
                return self.get_frame(i).map(Some);
            }
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::writer::{GhostlineWriter, Header};
    use std::io::Cursor;

    fn write_test_frames() -> Vec<u8> {
        let mut buf = Vec::new();
        let header = Header {
            started_at: 1700000000000,
            git_sha: None,
        };
        let mut writer = GhostlineWriter::new(&mut buf, &header).unwrap();

        for i in 0..3 {
            let frame = Frame::new(
                format!("request-{}", i).into_bytes(),
                format!("response-{}", i).into_bytes(),
                10 + i as u64,
                1700000000000 + i as u64,
            );
            writer.append(&frame).unwrap();
        }
        writer.finish().unwrap();
        buf
    }

    #[test]
    fn read_frame_count() {
        let buf = write_test_frames();
        let reader = GhostlineReader::from_reader(Cursor::new(buf)).unwrap();
        assert_eq!(reader.frame_count(), 3);
    }

    #[test]
    fn read_all_frames() {
        let buf = write_test_frames();
        let mut reader = GhostlineReader::from_reader(Cursor::new(buf)).unwrap();
        for i in 0..3 {
            let frame = reader.get_frame(i).unwrap();
            assert_eq!(frame.request_bytes, format!("request-{}", i).into_bytes());
            assert_eq!(frame.response_bytes, format!("response-{}", i).into_bytes());
            assert_eq!(frame.latency_ms, 10 + i as u64);
        }
    }

    #[test]
    fn lookup_by_hash_works() {
        let buf = write_test_frames();
        let mut reader = GhostlineReader::from_reader(Cursor::new(buf)).unwrap();

        let expected_hash = Frame::hash_request(b"request-1");
        let frame = reader.lookup_by_hash(&expected_hash).unwrap().unwrap();
        assert_eq!(frame.request_bytes, b"request-1");
    }

    #[test]
    fn lookup_by_hash_not_found() {
        let buf = write_test_frames();
        let mut reader = GhostlineReader::from_reader(Cursor::new(buf)).unwrap();
        let fake_hash = [0u8; 32];
        assert!(reader.lookup_by_hash(&fake_hash).unwrap().is_none());
    }

    #[test]
    fn roundtrip_request_bytes() {
        let originals: Vec<Vec<u8>> = (0..3)
            .map(|i| format!("request-{}", i).into_bytes())
            .collect();

        let buf = write_test_frames();
        let mut reader = GhostlineReader::from_reader(Cursor::new(buf)).unwrap();

        for i in 0..3 {
            let frame = reader.get_frame(i).unwrap();
            assert_eq!(frame.request_bytes, originals[i]);
        }
    }
}
