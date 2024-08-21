// Copyright 2022 System76 <info@system76.com>
// SPDX-License-Identifier: MPL-2.0

//! Parse the `~/.local/share/recently-used.xbel` file
//!
//! ```
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let recently_used = recently_used_xbel::parse_file()?;
//!
//!     for bookmark in recently_used.bookmarks {
//!         println!("{:?}", bookmark);
//!     }
//!
//!     Ok(())
//! }
//! ```

use chrono::{DateTime, Utc};
use quick_xml::se::to_string as quick_to_string;
use quick_xml::DeError;
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
    time::SystemTime,
};
use url::Url;

/// Stores recently-opened files accessed by the desktop user.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename = "xbel", rename_all = "kebab-case")]
pub struct RecentlyUsed {
    /// Files that have been recently used.
    #[serde(rename = "bookmark")]
    pub bookmarks: Vec<Bookmark>,
}

/// A file that was recently opened by the desktop user.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Bookmark {
    /// The location of the file.
    #[serde(rename = "@href")]
    pub href: String,
    /// When the file was added to the list.
    #[serde(rename = "@added")]
    pub added: String,
    /// When the file was last modified.
    #[serde(rename = "@modified")]
    pub modified: String,
    /// When the file was last visited.
    #[serde(rename = "@visited")]
    pub visited: String,
}

/// An error that can occur when accessing recently-used files.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("~/.local/share/recently-used.xbel: file does not exist")]
    DoesNotExist,
    #[error("~/.local/share/recently-used.xbel: could not deserialize")]
    Deserialization(#[source] DeError),
    #[error("could not serialize new file")]
    Serialization(#[source] DeError),
    #[error("could not read recents file")]
    Read(#[source] std::io::Error),
    #[error("could not read metadata from path")]
    Metadata(#[source] std::io::Error),
    #[error("could not read generate href from path")]
    Path,
    #[error("could not update recent files")]
    Update,
}

/// The path where the recently-used.xbel file is expected to be found.
pub fn dir() -> Option<PathBuf> {
    dirs::home_dir().map(|dir| dir.join(".local/share/recently-used.xbel"))
}

/// Convenience function for parsing the recently-used.xbel file in its default location.
pub fn parse_file() -> Result<RecentlyUsed, Error> {
    let path = dir().ok_or(Error::DoesNotExist)?;
    let file_content = fs::read_to_string(&path).map_err(|err| Error::Read(err))?;
    quick_xml::de::from_str(&file_content).map_err(|err| Error::Deserialization(err))
}

/// Updates the list of recently used files.
///
/// This function checks if the specified file already exists in the recently used list.
/// If it exists, the function updates the file's metadata (such as added, modified, and visited).
/// If it does not exist, the function adds a new entry for the file.
///
/// # Arguments
///
/// * `element_path` - A `PathBuf` that represents the path to the file being updated or added.
///
/// # Returns
///
/// This function returns `Result<(), Error>`, which is:
/// - `Ok(())` on success.
/// - `Err(Error)` if there is a failure in processing the file (e.g., reading metadata, serialization, or file I/O).
///
/// # Errors
///
/// This function can return errors in the following cases:
///
/// - If the file's metadata cannot be accessed or read.
/// - If the recently used file list cannot be parsed or serialized.
/// - If there is an issue writing the updated list back to the file system.
pub fn update_recently_used(element_path: &PathBuf) -> Result<(), Error> {
    let mut parsed_file = parse_file()?;

    let metadata = element_path.metadata().map_err(Error::Metadata)?;
    let added = system_time_to_string(metadata.created().map_err(Error::Metadata)?);
    let modified = system_time_to_string(metadata.modified().map_err(Error::Metadata)?);
    let visited = system_time_to_string(metadata.accessed().map_err(Error::Metadata)?);
    let href = path_to_href(element_path).ok_or(Error::Path)?;

    let bookmark = Bookmark {
        href,
        added,
        modified,
        visited,
    };

    // Remove the old bookmark if it exists, and add the updated one
    parsed_file.bookmarks.retain(|b| b.href != bookmark.href);
    parsed_file.bookmarks.push(bookmark);

    let serialized = quick_to_string(&parsed_file).map_err(Error::Serialization)?;

    let recently_used_file_path = dir().ok_or(Error::DoesNotExist)?;

    let mut file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(recently_used_file_path)
        .map_err(|_| Error::Update)?;

    file.write_all(serialized.as_bytes())
        .map_err(|_| Error::Update)?;

    Ok(())
}

fn system_time_to_string(time: SystemTime) -> String {
    let datetime: DateTime<Utc> = time.into();
    // Format the DateTime as a string ISO 8601
    datetime.to_rfc3339()
}

fn path_to_href(path: &PathBuf) -> Option<String> {
    let path_str = path.to_str()?;
    Url::from_file_path(path_str)
        .ok()
        .map(|url| url.into_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_update_recenty_used() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = tempdir()?;
        let temp_file_path = temp_dir.path().join("test_file.txt");
        let recently_used_path = dir().ok_or(Error::DoesNotExist)?;

        fs::write(&temp_file_path, b"Test content")?;

        if !recently_used_path.exists() {
            create_empty_recently_used_file(&recently_used_path)?;
        }

        update_recently_used(&temp_file_path)?;

        let content = fs::read_to_string(recently_used_path)?;
        assert!(content.contains("test_file.txt"));
        Ok(())
    }

    fn create_empty_recently_used_file(path: &PathBuf) -> Result<(), Error> {
        let empty_file = RecentlyUsed { bookmarks: vec![] };
        let serialized = quick_to_string(&empty_file).map_err(Error::Serialization)?;
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .open(path)
            .map_err(|_| Error::Update)?;
        file.write_all(serialized.as_bytes())
            .map_err(|_| Error::Update)?;
        Ok(())
    }
}
