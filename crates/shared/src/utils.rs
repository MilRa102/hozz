use sha2::{Digest, Sha256};

#[allow(clippy::cast_precision_loss)]
#[allow(clippy::must_use_candidate)]
pub fn format_size(bytes: i64) -> String {
    const KB: f64 = 1024.0;
    let b = bytes as f64;
    if b < KB {
        return format!("{b} B");
    }
    let kb = b / KB;
    if kb < KB {
        return format!("{kb:.2} KB");
    }
    let mb = kb / KB;
    if mb < KB {
        return format!("{mb:.2} MB");
    }
    format!("{:.2} GB", mb / KB)
}

#[allow(clippy::cast_precision_loss)]
#[allow(clippy::must_use_candidate)]
pub fn format_bytes(bytes: u64) -> String {
    let kib = 1024.0;
    let mib = kib * 1024.0;
    let gib = mib * 1024.0;
    let b = bytes as f64;

    if b >= gib {
        format!("{:.2} GB/s", b / gib)
    } else if b >= mib {
        format!("{:.1} MB/s", b / mib)
    } else if b >= kib {
        format!("{:.0} KB/s", b / kib)
    } else {
        format!("{b} B/s")
    }
}

/// Generates a unique short code by value
///
/// # Arguments
/// * `value` - The name by which to create a hash
///
/// # Returns
/// String as a hash
///
/// # Example
/// ```
/// use shared::utils::generate_id();
/// let hash = generate_id("example");
/// assert_eq!(hash.len(), 16);
/// ```
#[must_use]
pub fn generate_id(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());

    let result = hasher.finalize();
    hex::encode(result)[..16].to_string()
}
