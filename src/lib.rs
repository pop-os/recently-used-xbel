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

use chrono::{DateTime, SecondsFormat, Utc};
use custom_writer::custom_write;
use quick_xml::DeError;
use serde::{Deserialize, Serialize};
use std::{
    fs::{self},
    path::PathBuf,
    time::SystemTime,
};
use url::Url;
mod custom_writer;

/// Stores recently-opened files accessed by the desktop user.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename = "xbel", rename_all = "kebab-case")]
pub struct RecentlyUsed {
    #[serde(rename = "@xmlns:bookmark")]
    pub xmlns_bookmark: String,
    #[serde(rename = "@xmlns:mime")]
    pub xmlns_mime: String,

    /// Files that have been recently used.
    #[serde(rename = "bookmark", default)]
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
    /// Additional metadata and applications related to the bookmark.
    #[serde(rename = "info")]
    pub info: Option<Info>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Info {
    /// Metadata about the bookmark.
    #[serde(rename = "metadata")]
    pub metadata: Metadata,
}

/// Metadata containing MIME type and application info.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Metadata {
    /// The owner of the metadata.
    #[serde(rename = "@owner")]
    pub owner: String,

    /// The MIME type information.
    #[serde(rename = "mime-type")]
    pub mime_type: Option<MimeType>,

    /// The applications that have accessed the file.
    #[serde(rename = "applications")]
    pub applications: Applications,
}

/// The MIME type of the file.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct MimeType {
    /// The type of the file (e.g., "text/markdown").
    #[serde(rename = "@type")]
    pub mime_type: String,
}

/// A list of applications that accessed the bookmark.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Applications {
    /// The list of applications.
    //#[serde(rename(deserialize="application", serialize="bookmark:applications"))]
    #[serde(rename = "application")]
    pub applications: Vec<Application>,
}

/// An application that accessed the bookmark.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Application {
    /// The name of the application.
    #[serde(rename = "@name")]
    pub name: String,

    /// The command used to execute the application.
    #[serde(rename = "@exec")]
    pub exec: String,

    /// When the application last modified the bookmark.
    #[serde(rename = "@modified")]
    pub modified: String,

    /// The number of times the application has accessed the bookmark.
    #[serde(rename = "@count")]
    pub count: u32,
}

/// An error that can occur when accessing recently-used files.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("~/.local/share/recently-used.xbel: file does not exist")]
    DoesNotExist,
    #[error("~/.local/share/recently-used.xbel: could not deserialize")]
    Deserialization(#[source] DeError),
    #[error("could not serialize new file")]
    Serialization(#[source] Option<DeError>),
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
/// If it exists, the function updates the file's metadata, including the times when the file was
/// added, modified, and last visited. If the file does not exist in the list, the function adds
/// a new entry for the file.
///
/// If the file already exists in the list, the function also updates the application's usage count,
/// or adds a new application entry if it hasn't been recorded previously.
///
/// # Arguments
///
/// * `element_path` - A `PathBuf` that represents the path to the file being updated or added.
/// * `app_name` - A `String` representing the name of the application associated with the file.
/// * `exec` - A `String` representing the command to execute the application.
/// * `owner` - An optional `String` representing the owner of the metadata. If not provided,
///   defaults to `"http://freedesktop.org"`.
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
pub fn update_recently_used(
    element_path: &PathBuf,
    app_name: String,
    exec: String,
    owner: Option<String>,
) -> Result<(), Error> {
    let owner = match owner {
        Some(owner) => owner,
        None => "http://freedesktop.org".to_string(),
    };
    let mut parsed_file = parse_file()?;
    let href = path_to_href(element_path).ok_or(Error::Path)?;
    let metadata = element_path.metadata().map_err(Error::Metadata)?;
    let added = system_time_to_string(metadata.created().map_err(Error::Metadata)?);
    let modified = system_time_to_string(metadata.modified().map_err(Error::Metadata)?);
    let visited = system_time_to_string(metadata.accessed().map_err(Error::Metadata)?);

    // Attempt to find the existing bookmark and update it if found
    let existing_bookmark = parsed_file.bookmarks.iter_mut().find(|b| b.href == href);

    if let Some(bookmark) = existing_bookmark {
        // Bookmark exists, update the metadata
        bookmark.added = added;
        bookmark.modified = modified.clone();
        bookmark.visited = visited;

        // Find the application entry or insert a new one
        if let Some(info) = bookmark.info.as_mut() {
            if let Some(app) = info
                .metadata
                .applications
                .applications
                .iter_mut()
                .find(|el| el.name == app_name)
            {
                app.count += 1;
                app.modified = modified.clone();
            } else {
                // Application not found, insert a new one
                info.metadata.applications.applications.push(Application {
                    name: app_name,
                    exec,
                    modified: modified.clone(),
                    count: 1,
                });
            }
        }
    } else {
        // Bookmark does not exist, create a new one
        let mime = mime_from_path(&element_path).map(|mime| MimeType { mime_type: mime });

        let applications = vec![Application {
            name: app_name,
            exec,
            modified: modified.clone(),
            count: 1,
        }];

        let info = Info {
            metadata: Metadata {
                owner,
                mime_type: mime,
                applications: Applications { applications },
            },
        };

        let new_bookmark = Bookmark {
            href,
            added,
            modified,
            visited,
            info: Some(info),
        };

        parsed_file.bookmarks.push(new_bookmark);
    }

    let serialized = custom_write(parsed_file.clone())?;
    let recently_used_file_path = dir().ok_or(Error::DoesNotExist)?;
    let xml_declaration = r#"<?xml version="1.0" encoding="UTF-8"?>"#;
    let full_content = format!("{}{}", xml_declaration, serialized);

    fs::write(recently_used_file_path, full_content).map_err(|_| Error::Update)?;

    Ok(())
}

fn system_time_to_string(time: SystemTime) -> String {
    let datetime: DateTime<Utc> = time.into();
    datetime.to_rfc3339_opts(SecondsFormat::Micros, true)
}

fn path_to_href(path: &PathBuf) -> Option<String> {
    let path_str = path.to_str()?;
    Url::from_file_path(path_str)
        .ok()
        .map(|url| url.into_string())
}

fn mime_from_path(path: &PathBuf) -> Option<String> {
    let path = path.to_string_lossy().to_string();
    println!("path to infer: {:?}", path);
    let kind = mime_guess::from_path(path);
    println!("mimetype: {:?}", kind);
    let mime = kind.first();
    let mime = match mime {
        Some(mime) => mime,
        None => return None,
    };
    Some(format!("{}/{}", mime.type_(), mime.subtype()))
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

        update_recently_used(
            &temp_file_path,
            String::from("org.test"),
            String::from("test"),
            None,
        )?;

        // check new file name is in recents
        let content = fs::read_to_string(recently_used_path)?;
        assert!(content.contains("test_file.txt"));

        let deserialized = parse_file()?;

        assert!(deserialized.bookmarks.len() > 0);

        let bookmark = deserialized
            .bookmarks
            .iter()
            .find(|el| el.href.contains("test_file"));

        assert!(bookmark.is_some());

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
