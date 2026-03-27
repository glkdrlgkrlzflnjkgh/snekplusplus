use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

pub fn load_snekpp_with_imports(
    path: &Path,
    import_dir: Option<&Path>,
    visited: &mut HashSet<PathBuf>,
) -> Result<String, String> {
    let canon = fs::canonicalize(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if !visited.insert(canon.clone()) {
        return Ok(String::new());
    }

    let content = fs::read_to_string(&canon).map_err(|e| format!("{}: {e}", canon.display()))?;

    let mut out = String::new();
    let dir = canon.parent().unwrap_or(Path::new("."));

    for line in content.lines() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("import ") {
            let rest = rest.trim_start();
            if let Some(rest) = rest.strip_prefix('"') {
                if let Some(end) = rest.find('"') {
                    let fname = &rest[..end];

                    let primary = dir.join(fname);
                    let mut tried = Vec::new();

                    let imported = if primary.exists() {
                        tried.push(primary.display().to_string());
                        load_snekpp_with_imports(&primary, import_dir, visited)
                    } else if let Some(extra) = import_dir {
                        let secondary = extra.join(fname);
                        tried.push(primary.display().to_string());
                        tried.push(secondary.display().to_string());
                        if secondary.exists() {
                            load_snekpp_with_imports(&secondary, import_dir, visited)
                        } else {
                            Err(format!(
                                "import \"{fname}\" not found. tried: {}",
                                tried.join(", ")
                            ))
                        }
                    } else {
                        tried.push(primary.display().to_string());
                        Err(format!(
                            "import \"{fname}\" not found. tried: {}",
                            tried.join(", ")
                        ))
                    }?;

                    out.push_str(&imported);
                    out.push('\n');
                    continue;
                }
            }
            return Err(format!("invalid import syntax: {line}"));
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }

    Ok(out)
}
