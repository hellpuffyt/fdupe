//! Content hashing used by cascade stages 2 (sample) and 3 (full).

use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;

const READ_CHUNK: usize = 64 * 1024;

/// Hash the first `sample_size` bytes and the last `sample_size` bytes of a
/// file. For files no larger than `2 * sample_size` this reads (and thus
/// hashes) the entire file, which is fine: it only happens for files that
/// already survived the free size-grouping stage.
///
/// # Errors
/// Returns an error if the file cannot be opened or read.
pub fn sample_hash(path: &Path, size: u64, sample_size: u64) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();

    if size <= sample_size.saturating_mul(2) {
        let mut buf = Vec::with_capacity(usize::try_from(size).unwrap_or(usize::MAX));
        file.read_to_end(&mut buf)?;
        hasher.update(&buf);
        return Ok(format!("{:x}", hasher.finalize()));
    }

    let mut head = vec![0u8; usize::try_from(sample_size).unwrap_or(usize::MAX)];
    file.read_exact(&mut head)?;
    hasher.update(&head);

    let tail_start = size.saturating_sub(sample_size);
    file.seek(SeekFrom::Start(tail_start))?;
    let mut tail = vec![0u8; usize::try_from(sample_size).unwrap_or(usize::MAX)];
    file.read_exact(&mut tail)?;
    hasher.update(&tail);

    Ok(format!("{:x}", hasher.finalize()))
}

/// Hash the entire contents of a file, streaming it in fixed-size chunks so
/// memory use stays bounded regardless of file size.
///
/// # Errors
/// Returns an error if the file cannot be opened or read.
pub fn full_hash(path: &Path) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; READ_CHUNK];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}
