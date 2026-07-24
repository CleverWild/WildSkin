//! Reads the `.text` section out of a PE file on disk, for compile-time
//! comparison against AOB signatures (the game need not be running).
//!
//! File-layout counterpart to `WildSkin-rs`'s `memory::scanner::text_section`,
//! which walks the same headers against a live image. Key difference: a loaded
//! image is relocated to `VirtualAddress`; an on-disk file uses
//! `PointerToRawData` (file-aligned, can differ from `VirtualAddress`).
//!
//! Uses bounds-checked byte reads, not unsafe struct-cast derefs: transmuting
//! an untrusted, arbitrarily-aligned file buffer into a `#[repr(C)]` ref is
//! UB-prone in a way loader-mapped memory isn't.

use std::io::{Error, ErrorKind, Result};
use std::path::Path;

fn invalid_data(msg: &str) -> Error {
    Error::new(ErrorKind::InvalidData, msg)
}

/// Reads a little-endian `u32` out of `data` at `offset`, bounds-checked.
fn read_u32(data: &[u8], offset: usize) -> Result<u32> {
    let bytes = data
        .get(offset..offset + 4)
        .ok_or_else(|| invalid_data("PE file truncated (u32 read out of bounds)"))?;
    Ok(u32::from_le_bytes(bytes.try_into().unwrap()))
}

/// Reads a little-endian `u16` out of `data` at `offset`, bounds-checked.
fn read_u16(data: &[u8], offset: usize) -> Result<u16> {
    let bytes = data
        .get(offset..offset + 2)
        .ok_or_else(|| invalid_data("PE file truncated (u16 read out of bounds)"))?;
    Ok(u16::from_le_bytes(bytes.try_into().unwrap()))
}

/// Reads a little-endian `i32` out of `data` at `offset`, bounds-checked.
fn read_i32(data: &[u8], offset: usize) -> Result<i32> {
    let bytes = data
        .get(offset..offset + 4)
        .ok_or_else(|| invalid_data("PE file truncated (i32 read out of bounds)"))?;
    Ok(i32::from_le_bytes(bytes.try_into().unwrap()))
}

/// Size of `IMAGE_FILE_HEADER`, right after the 4-byte `PE\0\0` signature.
const IMAGE_FILE_HEADER_SIZE: usize = 20;
/// Size in bytes of a single `IMAGE_SECTION_HEADER` entry.
const IMAGE_SECTION_HEADER_SIZE: usize = 40;
/// The 8-byte, NUL-padded section name PE uses for the code section.
const TEXT_SECTION_NAME: &[u8; 8] = b".text\0\0\0";

/// Reads the `.text` section's raw bytes out of a PE file on disk.
///
/// Walks `IMAGE_DOS_HEADER` -> `IMAGE_NT_HEADERS64` -> section table like
/// `memory::scanner::text_section`, but using `PointerToRawData`/
/// `SizeOfRawData` (on-disk) not `VirtualAddress` (in-memory).
pub fn read_text_section(exe_path: &Path) -> Result<Vec<u8>> {
    let data = std::fs::read(exe_path)?;

    if data.get(0..2) != Some(b"MZ") {
        return Err(invalid_data("not a valid PE file (missing MZ signature)"));
    }

    // `e_lfanew`: file offset of the NT headers, at DOS header offset 0x3C.
    let e_lfanew = read_i32(&data, 0x3C)?;
    let nt_offset = usize::try_from(e_lfanew)
        .map_err(|_| invalid_data("PE file has a negative e_lfanew offset"))?;

    if data.get(nt_offset..nt_offset + 4) != Some(b"PE\0\0") {
        return Err(invalid_data(
            "not a valid PE file (missing PE\\0\\0 signature)",
        ));
    }

    // IMAGE_FILE_HEADER starts right after the 4-byte NT signature.
    let file_header_offset = nt_offset + 4;
    let number_of_sections = read_u16(&data, file_header_offset + 2)?;
    let size_of_optional_header = read_u16(&data, file_header_offset + 16)?;

    // Section table follows the optional header, which follows IMAGE_FILE_HEADER.
    let section_table_offset =
        file_header_offset + IMAGE_FILE_HEADER_SIZE + size_of_optional_header as usize;

    for i in 0..number_of_sections as usize {
        let section_offset = section_table_offset + i * IMAGE_SECTION_HEADER_SIZE;
        let name = data
            .get(section_offset..section_offset + 8)
            .ok_or_else(|| invalid_data("PE file truncated (section header out of bounds)"))?;
        if name != TEXT_SECTION_NAME {
            continue;
        }

        // Layout within IMAGE_SECTION_HEADER: Name(8) + Misc/union(4) then
        // VirtualAddress(4), SizeOfRawData(4), PointerToRawData(4).
        let virtual_address_offset = section_offset + 12;
        let pointer_to_raw_data = read_u32(&data, virtual_address_offset + 8)? as usize;
        let size_of_raw_data = read_u32(&data, virtual_address_offset + 4)? as usize;

        let end = pointer_to_raw_data
            .checked_add(size_of_raw_data)
            .ok_or_else(|| invalid_data(".text section bounds overflow"))?;
        let section_bytes = data
            .get(pointer_to_raw_data..end)
            .ok_or_else(|| invalid_data(".text section extends past end of file"))?;
        return Ok(section_bytes.to_vec());
    }

    Err(invalid_data("no .text section found"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a minimal but structurally valid synthetic PE buffer with a
    /// single section named `section_name`, whose raw data is `section_data`.
    fn make_synthetic_pe(section_name: [u8; 8], section_data: &[u8]) -> Vec<u8> {
        let mut buf = Vec::new();

        // DOS header: "MZ" + padding up to 0x3C, then e_lfanew.
        buf.extend_from_slice(b"MZ");
        buf.resize(0x3C, 0);
        let nt_offset = buf.len() as i32 + 4; // e_lfanew points right after itself
        buf.extend_from_slice(&nt_offset.to_le_bytes());
        assert_eq!(buf.len(), nt_offset as usize);

        // NT signature + IMAGE_FILE_HEADER.
        buf.extend_from_slice(b"PE\0\0");
        let size_of_optional_header: u16 = 0xF0;
        buf.extend_from_slice(&0u16.to_le_bytes()); // Machine
        buf.extend_from_slice(&1u16.to_le_bytes()); // NumberOfSections = 1
        buf.extend_from_slice(&0u32.to_le_bytes()); // TimeDateStamp
        buf.extend_from_slice(&0u32.to_le_bytes()); // PointerToSymbolTable
        buf.extend_from_slice(&0u32.to_le_bytes()); // NumberOfSymbols
        buf.extend_from_slice(&size_of_optional_header.to_le_bytes());
        buf.extend_from_slice(&0u16.to_le_bytes()); // Characteristics
        assert_eq!(buf.len() - (nt_offset as usize + 4), IMAGE_FILE_HEADER_SIZE);

        // Optional header body: irrelevant contents, just skip over it.
        buf.resize(buf.len() + size_of_optional_header as usize, 0);

        // Single IMAGE_SECTION_HEADER for `.text`.
        let pointer_to_raw_data = (buf.len() + IMAGE_SECTION_HEADER_SIZE + 4) as u32;
        buf.extend_from_slice(&section_name);
        buf.extend_from_slice(&0u32.to_le_bytes()); // Misc/union (unused)
        buf.extend_from_slice(&0u32.to_le_bytes()); // VirtualAddress (unused here)
        buf.extend_from_slice(&(section_data.len() as u32).to_le_bytes()); // SizeOfRawData
        buf.extend_from_slice(&pointer_to_raw_data.to_le_bytes()); // PointerToRawData
        buf.extend_from_slice(&0u32.to_le_bytes()); // PointerToRelocations
        buf.extend_from_slice(&0u32.to_le_bytes()); // PointerToLinenumbers
        buf.extend_from_slice(&0u16.to_le_bytes()); // NumberOfRelocations
        buf.extend_from_slice(&0u16.to_le_bytes()); // NumberOfLinenumbers
        buf.extend_from_slice(&0u32.to_le_bytes()); // Characteristics

        // A few bytes of further padding, then the marker bytes at
        // `pointer_to_raw_data`.
        buf.resize(pointer_to_raw_data as usize, 0);
        buf.extend_from_slice(section_data);

        buf
    }

    /// Writes `bytes` to a unique temp file and returns its path; the caller
    /// is responsible for removing it.
    fn write_temp_file(name: &str, bytes: &[u8]) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "abi-verify-pe-test-{}-{}-{name}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::write(&path, bytes).unwrap();
        path
    }

    #[test]
    fn reads_text_section_bytes_from_a_synthetic_pe_file() {
        let marker = [0xDEu8, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE];
        let pe = make_synthetic_pe(*b".text\0\0\0", &marker);
        let path = write_temp_file("ok", &pe);

        let result = read_text_section(&path);
        let _ = std::fs::remove_file(&path);

        assert_eq!(result.unwrap(), marker.to_vec());
    }

    #[test]
    fn rejects_a_file_missing_the_mz_signature() {
        let path = write_temp_file("no-mz", &[0u8, 0, 0, 0]);
        let result = read_text_section(&path);
        let _ = std::fs::remove_file(&path);

        result.unwrap_err();
    }

    #[test]
    fn rejects_a_pe_file_with_no_text_section() {
        let pe = make_synthetic_pe(*b".data\0\0\0", &[0x11, 0x22]);
        let path = write_temp_file("no-text", &pe);

        let result = read_text_section(&path);
        let _ = std::fs::remove_file(&path);

        let err = result.unwrap_err();
        assert_eq!(err.kind(), ErrorKind::InvalidData);
        assert!(err.to_string().contains(".text"));
    }
}
