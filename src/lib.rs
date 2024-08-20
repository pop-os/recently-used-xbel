// Copyright 2022 System76 <info@system76.com>
// SPDX-License-Identifier: MPL-2.0

//! Parse the `~/.local/share/recently-used.xbel` file
//!
//! ```
//! let recently_used = match recently_used_xbel::parse_file()?;
//!
//! for bookmark in recently_used.bookmarks {
//!     println!("{:?}", bookmark);
//! }
//! ```

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_xml_rs::to_string;
use std::{fs::OpenOptions, io::Write, path::PathBuf, time::SystemTime};
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
#[serde(rename = "bookmark")]
pub struct Bookmark {
    /// The location of the file.
    pub href: String,
    /// When the file was added to the list.
    pub added: String,
    /// When the file was last modified.
    pub modified: String,
    /// When the file was last visited.
    pub visited: String,
}

/// An error that can occur when accessing recently-used files.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("~/.local/share/recently-used.xbel: file does not exist")]
    DoesNotExist,
    #[error("~/.local/share/recently-used.xbel: could not deserialize")]
    Deserialization(#[source] serde_xml_rs::Error),
    #[error("could not serialize new file")]
    Serialization(#[source] serde_xml_rs::Error),
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
    let file = std::fs::File::open(&*path).map_err(|_| Error::DoesNotExist)?;
    serde_xml_rs::from_reader(file).map_err(Error::Deserialization)
}

/// Function to update a recently used file.
/// It check if the file exist in the list and udate it,
/// otherwise add it
pub fn update_recenty_used(element_path: &PathBuf) -> Result<(), Error> {
    let mut parsed_file = parse_file()?;

    let metadata = element_path
        .metadata()
        .map_err(|err| Error::Metadata(err))?;
    let added = system_time_to_string(metadata.created().map_err(|err| Error::Metadata(err))?);
    let modified = system_time_to_string(metadata.modified().map_err(|err| Error::Metadata(err))?);
    let visited = system_time_to_string(metadata.accessed().map_err(|err| Error::Metadata(err))?);
    let href = path_to_href(element_path).ok_or(Error::Path)?;

    let bookmark = Bookmark {
        href,
        added,
        modified,
        visited,
    };

    parsed_file.bookmarks.push(bookmark);

    let serialized = to_string(&parsed_file).map_err(Error::Serialization)?;

    let recenty_used_file_path = match dir() {
        Some(path) => path,
        None => return Err(Error::DoesNotExist),
    };

    let mut file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(recenty_used_file_path)
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

        update_recenty_used(&temp_file_path)?;

        let content = fs::read_to_string(recently_used_path)?;
        assert!(content.contains("file://"));
        Ok(())
    }

    fn create_empty_recently_used_file(path: &PathBuf) -> Result<(), Error> {
        let empty_file = RecentlyUsed { bookmarks: vec![] };
        let serialized = to_string(&empty_file).map_err(Error::Serialization)?;
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
