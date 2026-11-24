use std::fs;
use std::path::{Path, PathBuf};

const SQLITE_HEADER: &[u8; 16] = b"SQLite format 3\0";

/// Pure-Rust reader for Firefox 136+ `Profile Groups/<StoreID>.sqlite` databases.
/// Reads `(name, resolved_profile_dir)` pairs from the `Profiles` table without
/// linking `libsqlite3`.
pub fn read_firefox_group_profiles(db_path: &Path, firefox_root: &Path) -> Result<Vec<(String, PathBuf)>, String> {
    let bytes = fs::read(db_path).map_err(|e| format!("Failed reading {}: {}", db_path.display(), e))?;
    parse_sqlite_profiles(&bytes, firefox_root)
}

pub fn parse_sqlite_profiles(bytes: &[u8], firefox_root: &Path) -> Result<Vec<(String, PathBuf)>, String> {
    if bytes.len() < 100 || &bytes[..16] != SQLITE_HEADER {
        return Err("Not a valid SQLite database".to_string());
    }
    let raw_page_size = u16::from_be_bytes([bytes[16], bytes[17]]) as usize;
    let page_size = if raw_page_size == 1 { 65536 } else { raw_page_size };
    if page_size < 512 || !page_size.is_power_of_two() || bytes.len() < page_size {
        return Err("Invalid SQLite page size".to_string());
    }

    // Scan table B-tree leaf pages (`0x0d`) for `Profiles` rows `(id, path, name, avatar, theme, ...)`
    let mut out = Vec::new();
    let page_count = bytes.len() / page_size;
    for page_idx in 0..page_count {
        let page_start = page_idx * page_size;
        let page = &bytes[page_start..page_start + page_size];
        let hdr_offset = if page_idx == 0 { 100 } else { 0 };
        if page.len() <= hdr_offset + 8 {
            continue;
        }
        if page[hdr_offset] != 0x0d {
            continue;
        }
        let cell_count = u16::from_be_bytes([page[hdr_offset + 3], page[hdr_offset + 4]]) as usize;
        let ptr_table_start = hdr_offset + 8;
        if ptr_table_start + cell_count * 2 > page.len() {
            continue;
        }
        for i in 0..cell_count {
            let cell_offset = u16::from_be_bytes([
                page[ptr_table_start + i * 2],
                page[ptr_table_start + i * 2 + 1],
            ]) as usize;
            if cell_offset >= page.len() {
                continue;
            }
            if let Some((rel_path, name)) = parse_profile_cell(&page[cell_offset..]) {
                if rel_path.starts_with("Profiles/") && !name.is_empty() {
                    out.push((name, firefox_root.join(rel_path)));
                }
            }
        }
    }
    Ok(out)
}

fn parse_profile_cell(buf: &[u8]) -> Option<(String, String)> {
    let (payload_len, n1) = read_varint(buf)?;
    let (_rowid, n2) = read_varint(&buf[n1..])?;
    let record_start = n1 + n2;
    let payload_len = usize::try_from(payload_len).ok()?;
    let record = buf.get(record_start..record_start + payload_len)?;

    let (hdr_len, mut pos) = read_varint(record)?;
    let hdr_len = usize::try_from(hdr_len).ok()?;
    if hdr_len > record.len() {
        return None;
    }

    let mut serial_types = Vec::new();
    while pos < hdr_len {
        let (st, used) = read_varint(&record[pos..hdr_len])?;
        serial_types.push(st);
        pos += used;
    }
    // `Profiles` table has columns: id (0), path (1), name (2), ...
    if serial_types.len() < 3 {
        return None;
    }

    let mut body_pos = hdr_len;
    let mut path_val = None;
    let mut name_val = None;

    for (col_idx, &st) in serial_types.iter().enumerate().take(3) {
        let len = serial_type_len(st)?;
        let bytes = record.get(body_pos..body_pos + len)?;
        body_pos += len;
        if st >= 13 && st % 2 == 1 {
            let s = std::str::from_utf8(bytes).ok()?.to_string();
            if col_idx == 1 {
                path_val = Some(s);
            } else if col_idx == 2 {
                name_val = Some(s);
            }
        }
    }

    Some((path_val?, name_val?))
}

fn serial_type_len(st: u64) -> Option<usize> {
    match st {
        0 | 8 | 9 => Some(0),
        1 => Some(1),
        2 => Some(2),
        3 => Some(3),
        4 => Some(4),
        5 => Some(6),
        6 | 7 => Some(8),
        n if n >= 12 && n % 2 == 0 => usize::try_from((n - 12) / 2).ok(),
        n if n >= 13 && n % 2 == 1 => usize::try_from((n - 13) / 2).ok(),
        _ => None,
    }
}

fn read_varint(buf: &[u8]) -> Option<(u64, usize)> {
    let mut val = 0u64;
    for (i, &b) in buf.iter().take(9).enumerate() {
        if i == 8 {
            val = (val << 8) | u64::from(b);
            return Some((val, 9));
        }
        val = (val << 7) | u64::from(b & 0x7f);
        if b & 0x80 == 0 {
            return Some((val, i + 1));
        }
    }
    None
}
