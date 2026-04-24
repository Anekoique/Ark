//! Internal helper: extract an embedded template file to disk.

use std::path::Path;

use crate::{
    error::{Error, Result},
    io::{WriteMode, write_file},
    templates::ARK_TEMPLATES,
};

/// Write the embedded template `<name>.md` (e.g. `PRD`, `PLAN`, `REVIEW`,
/// `VERIFY`, `SPEC`) to `to`, overwriting if present.
pub(crate) fn copy_template(name: &str, to: &Path) -> Result<()> {
    let rel = format!("templates/{name}.md");
    let file = ARK_TEMPLATES
        .get_file(&rel)
        .ok_or_else(|| Error::UnknownTemplate {
            name: name.to_string(),
        })?;
    write_file(to, file.contents(), WriteMode::Force)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::PathExt;

    #[test]
    fn copies_known_template() {
        let tmp = tempfile::tempdir().unwrap();
        let dst = tmp.path().join("out/PRD.md");
        copy_template("PRD", &dst).unwrap();
        let text = dst.read_bytes().unwrap();
        assert!(!text.is_empty());
        assert!(String::from_utf8_lossy(&text).contains("PRD"));
    }

    #[test]
    fn errors_on_unknown_template() {
        let tmp = tempfile::tempdir().unwrap();
        let err = copy_template("NOPE", &tmp.path().join("nope.md")).unwrap_err();
        assert!(matches!(err, Error::UnknownTemplate { .. }));
    }
}
