// Copyright 2024 System76 <info@system76.com>
// SPDX-License-Identifier: MPL-2.0

use crate::RecentlyUsed;
use quick_xml::writer::Writer;
use quick_xml::Error;
use std::io::Cursor;

pub fn custom_write(recently_used: RecentlyUsed) -> Result<String, crate::Error> {
    let mut writer = Writer::new(Cursor::new(Vec::new()));

    let _ = writer
        .create_element("xbel")
        .with_attributes(
            vec![
                ("version", "1.0"),
                (
                    "xmlns:bookmark",
                    "http://www.freedesktop.org/standards/desktop-bookmarks",
                ),
                (
                    "xmlns:mime",
                    "http://www.freedesktop.org/standards/shared-mime-info",
                ),
            ]
            .into_iter(),
        )
        .write_inner_content::<_, Error>(|writer| {
            for b in recently_used.bookmarks {
                let _ = writer
                    .create_element("bookmark")
                    .with_attributes([
                        ("href", b.href.as_str()),
                        ("added", b.added.as_str()),
                        ("modified", b.modified.as_str()),
                        ("visited", b.visited.as_str()),
                    ])
                    .write_inner_content::<_, Error>(|writer| {
                        if let Some(info) = b.info {
                            let _ = writer
                                .create_element("info")
                                .write_inner_content::<_, Error>(|writer| {
                                    let _ = writer
                                        .create_element("metadata")
                                        .with_attributes([("owner", info.metadata.owner.as_str())])
                                        .write_inner_content::<_, Error>(|writer| {
                                            if let Some(mime) = info.metadata.mime_type {
                                                let _ = writer
                                                    .create_element("mime:mime-type")
                                                    .with_attributes([(
                                                        "type",
                                                        mime.mime_type.as_str(),
                                                    )])
                                                    .write_empty();
                                            }
                                            let _ = writer
                                                .create_element("bookmark:applications")
                                                .write_inner_content::<_, Error>(|writer| {
                                                for app in info.metadata.applications.applications {
                                                    let _ = writer
                                                        .create_element("bookmark:application")
                                                        .with_attributes([
                                                            ("name", app.name.as_str()),
                                                            ("exec", app.exec.as_str()),
                                                            ("modified", app.modified.as_str()),
                                                            (
                                                                "count",
                                                                app.count.to_string().as_str(),
                                                            ),
                                                        ])
                                                        .write_empty();
                                                }
                                                Ok(())
                                            });
                                            Ok(())
                                        });
                                    Ok(())
                                });
                        }
                        Ok(())
                    });
            }
            Ok(())
        });

    let bytes = writer.into_inner().into_inner();
    match String::from_utf8(bytes) {
        Ok(string) => Ok(string),
        Err(_e) => Err(crate::Error::Serialization(None)),
    }
}
