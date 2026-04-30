//! Managed-block editing for `<!-- NAME:START -->...<!-- NAME:END -->` regions
//! embedded in text files like `CLAUDE.md`.

use std::path::Path;

use crate::{
    error::{Error, Result},
    io::path_ext::PathExt,
};

/// Reads a managed block body from `path`.
///
/// Returns `Ok(None)` if the file or the markers are missing.
pub fn read_managed_block(path: impl AsRef<Path>, marker: &str) -> Result<Option<String>> {
    Ok(path
        .as_ref()
        .read_text_optional()?
        .and_then(|text| Marker::new(marker).extract_body(&text)))
}

/// Replaces a managed block body in `text`.
///
/// Returns `None` if the marker pair is not present.
pub fn splice_managed_block(text: &str, marker: &str, body: &str) -> Option<String> {
    Marker::new(marker).replace_in(text, body)
}

/// Returns every complete `ARK:*` marker pair in document order.
///
/// Names are de-duplicated. Used by upgrade to discover which regions of a
/// desired template should be reconciled against on-disk content.
pub fn scan_managed_markers(text: &str) -> Vec<String> {
    const PREFIX: &str = "<!-- ARK:";
    const END: &str = ":END -->";
    let mut names = Vec::new();
    let mut rest = text;
    while let Some(pos) = rest.find(PREFIX) {
        let tail = &rest[pos + PREFIX.len()..];
        let Some(colon) = tail.find(":START -->") else {
            break;
        };
        let name = &tail[..colon];
        let full = format!("ARK:{name}");
        // Only accept pairs: confirm a matching END appears later in the file.
        if text.contains(&format!("<!-- {full}{END}")) && !names.contains(&full) {
            names.push(full);
        }
        rest = &tail[colon..];
    }
    names
}

/// Inserts or replaces a delimited managed block in a text file.
///
/// Creates the file if it does not exist. Returns `true` once written.
///
/// # Errors
///
/// Returns [`Error::ManagedBlockCorrupt`] if the file contains a START
/// marker without a matching END — appending a fresh block in that case would
/// silently duplicate the marker and yield garbled state on subsequent reads.
pub fn update_managed_block(path: impl AsRef<Path>, marker: &str, body: &str) -> Result<bool> {
    let path = path.as_ref();
    let m = Marker::new(marker);
    let block = m.render(body);
    let new_contents = match path.read_text_optional()? {
        None => block,
        Some(text) => match m.replace_in(&text, body) {
            Some(replaced) => replaced,
            None if text.contains(&m.start()) => {
                return Err(Error::ManagedBlockCorrupt {
                    path: path.to_path_buf(),
                    marker: marker.to_string(),
                });
            }
            None => append_block(&text, &block),
        },
    };
    path.write_bytes(new_contents.as_bytes())?;
    Ok(true)
}

/// Splices on-disk managed-block bodies into `template`.
///
/// Returns the template bytes unchanged if `path` does not exist or if the
/// template carries no managed blocks.
///
/// This lets `init` and `upgrade` write embedded templates without clobbering
/// managed-block content that other commands (e.g. `spec register`) wrote
/// into the live file. The block bodies in the embedded template act as
/// fallbacks; the on-disk body always wins when present.
pub fn merge_managed_blocks(path: impl AsRef<Path>, template: &[u8]) -> Result<Vec<u8>> {
    let path = path.as_ref();
    let Ok(text) = std::str::from_utf8(template) else {
        return Ok(template.to_vec());
    };
    let markers = scan_managed_markers(text);
    if markers.is_empty() {
        return Ok(template.to_vec());
    }
    let Some(on_disk) = path.read_text_optional()? else {
        return Ok(template.to_vec());
    };
    let mut spliced = text.to_string();
    for marker in &markers {
        if let Some(body) = extract_block_body_for_splice(&on_disk, marker)
            && let Some(new_text) = splice_managed_block(&spliced, marker, body)
        {
            spliced = new_text;
        }
    }
    Ok(spliced.into_bytes())
}

/// Extracts a managed block body for splice round-trips.
///
/// Trims exactly one leading and one trailing `\n`. Interior blank lines are
/// preserved byte-identically.
fn extract_block_body_for_splice<'a>(text: &'a str, marker: &str) -> Option<&'a str> {
    let m = Marker::new(marker);
    let span = m.locate(text)?;
    let body = &text[span.body];
    let body = body.strip_prefix('\n').unwrap_or(body);
    let body = body.strip_suffix('\n').unwrap_or(body);
    Some(body)
}

/// Removes a managed block from a text file if present.
///
/// Deletes the file if the resulting content would be effectively empty.
/// Returns `true` if the block was present and removed.
pub fn remove_managed_block(path: impl AsRef<Path>, marker: &str) -> Result<bool> {
    let path = path.as_ref();
    let Some(stripped) = path
        .read_text_optional()?
        .and_then(|text| Marker::new(marker).strip_from(&text))
    else {
        return Ok(false);
    };
    if stripped.trim().is_empty() {
        path.remove_if_exists()?;
    } else {
        path.write_bytes(stripped.as_bytes())?;
    }
    Ok(true)
}

fn append_block(text: &str, block: &str) -> String {
    let sep = if text.is_empty() || text.ends_with('\n') {
        ""
    } else {
        "\n"
    };
    format!("{text}{sep}\n{block}")
}

/// Represents a managed-block delimiter pair.
///
/// The rendered delimiters are `<!-- NAME:START -->` and `<!-- NAME:END -->`.
/// This is an internal helper for the managed-block functions above.
#[derive(Debug, Clone, Copy)]
struct Marker<'a> {
    name: &'a str,
}

impl<'a> Marker<'a> {
    const fn new(name: &'a str) -> Self {
        Self { name }
    }

    fn start(&self) -> String {
        format!("<!-- {}:START -->", self.name)
    }

    fn end(&self) -> String {
        format!("<!-- {}:END -->", self.name)
    }

    fn render(&self, body: &str) -> String {
        format!("{}\n{}\n{}\n", self.start(), body, self.end())
    }

    fn extract_body(&self, text: &str) -> Option<String> {
        let span = self.locate(text)?;
        Some(text[span.body].trim_matches('\n').to_string())
    }

    fn replace_in(&self, text: &str, body: &str) -> Option<String> {
        let span = self.locate(text)?;
        Some(format!(
            "{prefix}{block}\n{suffix}",
            prefix = &text[..span.start],
            block = self.render(body).trim_end_matches('\n'),
            suffix = &text[span.end..],
        ))
    }

    fn strip_from(&self, text: &str) -> Option<String> {
        let span = self.locate(text)?;
        let before = text[..span.start].trim_end_matches('\n');
        let after = text[span.end..].trim_start_matches('\n');
        Some(match (before.is_empty(), after.is_empty()) {
            (true, true) => String::new(),
            (true, false) => format!("{after}\n"),
            (false, true) => format!("{before}\n"),
            (false, false) => format!("{before}\n{after}"),
        })
    }

    fn locate(&self, text: &str) -> Option<MarkerSpan> {
        let start = text.find(&self.start())?;
        let rel_end = text[start..].find(&self.end())? + start;
        let body_start = start + self.start().len();
        let line_end = text[rel_end..]
            .find('\n')
            .map_or(text.len(), |i| rel_end + i + 1);
        Some(MarkerSpan {
            start,
            end: line_end,
            body: body_start..rel_end,
        })
    }
}

struct MarkerSpan {
    start: usize,
    end: usize,
    body: std::ops::Range<usize>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn managed_block_insert_and_replace() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), "hello\n").unwrap();
        update_managed_block(tmp.path(), "ARK", "first").unwrap();
        let t1 = std::fs::read_to_string(tmp.path()).unwrap();
        assert!(t1.contains("first"));

        update_managed_block(tmp.path(), "ARK", "second").unwrap();
        let t2 = std::fs::read_to_string(tmp.path()).unwrap();
        assert!(t2.contains("second"));
        assert!(!t2.contains("first"));
    }

    #[test]
    fn managed_block_remove_deletes_file_when_only_block() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), "<!-- ARK:START -->\nbody\n<!-- ARK:END -->\n").unwrap();
        assert!(remove_managed_block(tmp.path(), "ARK").unwrap());
        assert!(!tmp.path().exists());
    }

    #[test]
    fn update_managed_block_errors_on_orphan_start() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), "<!-- ARK:START -->\nbody\nno-end-here\n").unwrap();
        let err = update_managed_block(tmp.path(), "ARK", "new body").unwrap_err();
        assert!(matches!(err, Error::ManagedBlockCorrupt { .. }));
    }

    #[test]
    fn read_managed_block_returns_body_or_none() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(
            tmp.path(),
            "before\n<!-- ARK:START -->\nfoo\nbar\n<!-- ARK:END -->\nafter\n",
        )
        .unwrap();
        assert_eq!(
            read_managed_block(tmp.path(), "ARK").unwrap().unwrap(),
            "foo\nbar"
        );

        std::fs::write(tmp.path(), "no markers here\n").unwrap();
        assert!(read_managed_block(tmp.path(), "ARK").unwrap().is_none());
    }
}
