//! Line-preserving 3DMigoto `.ini` parser and writer (Phase 13).
//!
//! Mod `.ini` files are INI-style: `[Section]` headers followed by
//! `key = value` lines, interspersed with `; comments` and blank lines. This
//! module parses them into an ordered structure that preserves **every** line
//! exactly as written — comments, blank lines, and original spacing — so that
//! a targeted edit (e.g. correcting one `hash =` value) rewrites only the one
//! line it touches and leaves the rest of the file byte-for-byte intact.
//!
//! This matters: mod authors and users keep meaningful comments and formatting
//! in these files, and a naive "parse to a map, serialize back" writer would
//! silently strip all of it. The `migrate_d3dx_ini_keys` helper in `mods.rs`
//! already follows this line-preserving principle; this generalises it.
//!
//! ## Safety
//! Any write goes through `write_line_edits`, which backs up the original
//! file before modifying it — to `<file>.bak`, or `<file>.bak.2`,
//! `<file>.bak.3`, … if a backup already exists, so repeated edits never
//! clobber an earlier original.

use std::fs;
use std::path::Path;

/// A single parsed line of an ini file, tagged by kind. The `raw` field always
/// holds the original text of the line (without its trailing newline) so the
/// file can be reconstructed exactly.
#[derive(Debug, Clone, PartialEq)]
pub enum IniLine {
    /// A `[Section]` header. `name` is the trimmed section name.
    Section { name: String, raw: String },
    /// A `key = value` pair. `key` and `value` are trimmed; `raw` is original.
    Pair { key: String, value: String, raw: String },
    /// A comment line (starts with `;` or `#` after optional whitespace).
    Comment { raw: String },
    /// A blank / whitespace-only line.
    Blank { raw: String },
    /// Anything that doesn't parse as the above (kept verbatim).
    Other { raw: String },
}

impl IniLine {
    /// The original text of this line (no trailing newline).
    pub fn raw(&self) -> &str {
        match self {
            IniLine::Section { raw, .. }
            | IniLine::Pair { raw, .. }
            | IniLine::Comment { raw }
            | IniLine::Blank { raw }
            | IniLine::Other { raw } => raw,
        }
    }
}

/// A parsed ini document: an ordered list of lines. Order and content are
/// preserved exactly so it can be written back unchanged.
#[derive(Debug, Clone)]
pub struct IniDoc {
    pub lines: Vec<IniLine>,
    /// True if the original file ended with a trailing newline, so we can
    /// reproduce that exactly on write.
    trailing_newline: bool,
}

/// Parse ini text into an ordered line list. Never fails — anything that
/// doesn't match a known line kind is preserved as `Other`.
pub fn parse(text: &str) -> IniDoc {
    let trailing_newline = text.ends_with('\n');
    let mut lines = Vec::new();

    for raw_line in text.split('\n') {
        // `split('\n')` on a trailing-newline string yields a final empty
        // element; skip that phantom so we don't invent a blank line. The
        // trailing newline is reproduced from the `trailing_newline` flag.
        // We only skip it when it's the very last element AND empty — handled
        // by collecting first.
        lines.push(classify_line(raw_line));
    }

    // If the text ended with '\n', the last split element is an empty string
    // that represents the newline itself, not a real blank line — drop it.
    if trailing_newline {
        lines.pop();
    }

    IniDoc {
        lines,
        trailing_newline,
    }
}

fn classify_line(raw: &str) -> IniLine {
    let trimmed = raw.trim();

    if trimmed.is_empty() {
        return IniLine::Blank { raw: raw.to_string() };
    }
    if trimmed.starts_with(';') || trimmed.starts_with('#') {
        return IniLine::Comment { raw: raw.to_string() };
    }
    if trimmed.starts_with('[') && trimmed.ends_with(']') && trimmed.len() >= 2 {
        let name = trimmed[1..trimmed.len() - 1].trim().to_string();
        return IniLine::Section {
            name,
            raw: raw.to_string(),
        };
    }
    // A key = value pair. Split on the first '='.
    if let Some(eq) = raw.find('=') {
        let key = raw[..eq].trim().to_string();
        let value = raw[eq + 1..].trim().to_string();
        if !key.is_empty() {
            return IniLine::Pair {
                key,
                value,
                raw: raw.to_string(),
            };
        }
    }
    IniLine::Other { raw: raw.to_string() }
}

impl IniDoc {
    /// Reconstruct the file text exactly as parsed.
    pub fn to_text(&self) -> String {
        let mut out = self
            .lines
            .iter()
            .map(|l| l.raw().to_string())
            .collect::<Vec<_>>()
            .join("\n");
        if self.trailing_newline {
            out.push('\n');
        }
        out
    }
}

/// One targeted edit: on `line_index` (0-based into the parsed line list),
/// replace the value of a `Pair` line, preserving the key and the original
/// spacing/indentation around the `=` as closely as practical.
#[derive(Debug, Clone)]
pub struct LineEdit {
    pub line_index: usize,
    pub new_value: String,
}

/// Apply a set of value edits to a parsed doc in memory. Each edit must target
/// a `Pair` line; edits to non-pair lines (or out-of-range indices) are
/// returned as errors and skipped, so a bad edit never corrupts the file.
///
/// Preserves the exact prefix of the original line up to and including the
/// `=` plus its following whitespace, then substitutes the new value — so
/// `  hash = old   ` becomes `  hash = new` with the leading indent and the
/// spacing around `=` retained.
pub fn apply_edits(doc: &mut IniDoc, edits: &[LineEdit]) -> Vec<String> {
    let mut errors = Vec::new();

    for edit in edits {
        let Some(line) = doc.lines.get_mut(edit.line_index) else {
            errors.push(format!("Line {} is out of range", edit.line_index));
            continue;
        };

        match line {
            IniLine::Pair { key, value, raw } => {
                let new_raw = rewrite_pair_raw(raw, &edit.new_value);
                *value = edit.new_value.clone();
                *raw = new_raw;
                let _ = key; // key unchanged
            }
            _ => {
                errors.push(format!(
                    "Line {} is not a key=value pair; refusing to edit",
                    edit.line_index
                ));
            }
        }
    }

    errors
}

/// Rebuild a pair's raw text with a new value, keeping everything up to and
/// including the `=` and the whitespace immediately after it. Any trailing
/// inline comment after the value is dropped intentionally (3DMigoto values
/// like hashes don't carry inline comments; keeping this simple avoids
/// mis-parsing `;` inside a value).
fn rewrite_pair_raw(raw: &str, new_value: &str) -> String {
    if let Some(eq) = raw.find('=') {
        // Everything through '='.
        let prefix = &raw[..=eq];
        // Preserve the run of spaces/tabs right after '='.
        let after = &raw[eq + 1..];
        let ws: String = after.chars().take_while(|c| *c == ' ' || *c == '\t').collect();
        format!("{}{}{}", prefix, ws, new_value)
    } else {
        // Shouldn't happen for a Pair, but fall back safely.
        raw.to_string()
    }
}

/// Write `edits` to the ini file at `path`, backing up the original first to a
/// non-colliding `.bak` name (see `next_backup_path`). Returns the list of
/// per-edit errors (empty = all applied). If no edits actually change anything,
/// no write and no backup occur.
pub fn write_line_edits(path: &Path, edits: &[LineEdit]) -> Result<Vec<String>, String> {
    let original = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;

    let mut doc = parse(&original);
    let errors = apply_edits(&mut doc, edits);

    let new_text = doc.to_text();
    if new_text == original {
        // Nothing changed (all edits were no-ops or failed) — don't touch disk.
        return Ok(errors);
    }

    // Backup before writing, to a name that doesn't clobber an existing
    // backup — so repeated edits keep every prior original.
    let bak = next_backup_path(path);
    fs::write(&bak, &original)
        .map_err(|e| format!("Failed to write backup {}: {}", bak.display(), e))?;

    fs::write(path, new_text)
        .map_err(|e| format!("Failed to write {}: {}", path.display(), e))?;

    Ok(errors)
}

/// Pick a non-colliding backup path for `path`. The first backup is
/// `<file>.bak`; if that already exists, fall back to `<file>.bak.2`,
/// `<file>.bak.3`, ... so an earlier backup is never overwritten.
///
/// e.g. `mod.ini` → `mod.ini.bak`, then `mod.ini.bak.2`, `mod.ini.bak.3`, …
fn next_backup_path(path: &Path) -> std::path::PathBuf {
    let base = {
        let mut s = path.as_os_str().to_os_string();
        s.push(".bak");
        std::path::PathBuf::from(s)
    };
    if !base.exists() {
        return base;
    }
    for n in 2.. {
        let mut s = path.as_os_str().to_os_string();
        s.push(format!(".bak.{}", n));
        let candidate = std::path::PathBuf::from(s);
        if !candidate.exists() {
            return candidate;
        }
    }
    unreachable!()
}

// ---------------------------------------------------------------------------
// Keybind + hash extraction (Phase 13, idea #1 and #2)
// ---------------------------------------------------------------------------

use serde::Serialize;

/// A keybinding declared in a `[Key*]` / `[KeySwap*]` section.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Keybind {
    /// The section name it was declared in (e.g. "KeySwap", "KeyToggleGlow").
    pub section: String,
    /// Raw `key =` value (e.g. "VK_DOWN", "no_modifiers VK_F"), if present.
    pub key: Option<String>,
    /// Friendly-translated form of `key` (e.g. "↓ Arrow").
    pub key_label: Option<String>,
    /// Raw `back =` value (the reverse-cycle key), if present.
    pub back: Option<String>,
    /// Friendly-translated form of `back`.
    pub back_label: Option<String>,
    /// The `type =` value (cycle / toggle / hold), if present.
    pub bind_type: Option<String>,
    /// The command-list variable this key drives (e.g. "$swapvar"), if the
    /// section assigns one with a value list like `$swapvar = 0,1,2`.
    pub variable: Option<String>,
    /// The value list assigned to `variable` (e.g. "0,1,2"), if present.
    pub values: Option<String>,
}

/// A `hash = <value>` occurrence, with enough info to target it for a write.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct HashEntry {
    /// The section the hash belongs to (e.g. "TextureOverrideFurinaBody").
    pub section: String,
    /// The current hash value.
    pub value: String,
    /// 0-based index of the line in the parsed doc — used to target the edit.
    pub line_index: usize,
}

/// The extracted, display-ready contents of a single ini file.
#[derive(Debug, Clone, Serialize)]
pub struct IniContents {
    pub keybinds: Vec<Keybind>,
    pub hashes: Vec<HashEntry>,
}

/// Whether a section name denotes a keybinding section. 3DMigoto uses `[Key...]`
/// and `[KeySwap...]` (case-insensitive in practice).
fn is_key_section(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.starts_with("key")
}

/// Extract keybinds and hashes from a parsed doc.
///
/// Walks lines in order, tracking the current section. Within a `[Key*]`
/// section it gathers `key`/`back`/`type` and the first `$var = a,b,c`
/// assignment (the driven variable). `hash =` lines are collected from any
/// section along with their line index for targeted rewriting.
pub fn extract(doc: &IniDoc) -> IniContents {
    let mut keybinds = Vec::new();
    let mut hashes = Vec::new();

    let mut current_section = String::new();
    // Accumulator for the in-progress key section.
    let mut pending: Option<Keybind> = None;

    // Helper to flush a pending keybind if it carries any useful info.
    fn flush(pending: &mut Option<Keybind>, out: &mut Vec<Keybind>) {
        if let Some(kb) = pending.take() {
            // Only keep it if it actually bound something.
            if kb.key.is_some() || kb.back.is_some() || kb.bind_type.is_some() {
                out.push(kb);
            }
        }
    }

    for (idx, line) in doc.lines.iter().enumerate() {
        match line {
            IniLine::Section { name, .. } => {
                flush(&mut pending, &mut keybinds);
                current_section = name.clone();
                if is_key_section(name) {
                    pending = Some(Keybind {
                        section: name.clone(),
                        key: None,
                        key_label: None,
                        back: None,
                        back_label: None,
                        bind_type: None,
                        variable: None,
                        values: None,
                    });
                }
            }
            IniLine::Pair { key, value, .. } => {
                let key_lower = key.to_ascii_lowercase();

                // Hash lines can appear in any section.
                if key_lower == "hash" && !value.is_empty() {
                    hashes.push(HashEntry {
                        section: current_section.clone(),
                        value: value.clone(),
                        line_index: idx,
                    });
                }

                if let Some(kb) = pending.as_mut() {
                    match key_lower.as_str() {
                        "key" => {
                            kb.key = Some(value.clone());
                            kb.key_label = Some(translate_key(value));
                        }
                        "back" => {
                            kb.back = Some(value.clone());
                            kb.back_label = Some(translate_key(value));
                        }
                        "type" => kb.bind_type = Some(value.clone()),
                        _ => {
                            // A `$var = 0,1,2` style assignment is the driven
                            // variable. Only capture the first one, and only if
                            // it looks like a command-list variable ($-prefixed)
                            // with a comma-separated value list.
                            if kb.variable.is_none()
                                && key.starts_with('$')
                                && value.contains(',')
                            {
                                kb.variable = Some(key.clone());
                                kb.values = Some(value.clone());
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
    flush(&mut pending, &mut keybinds);

    IniContents { keybinds, hashes }
}

/// Translate a 3DMigoto `key =` / `back =` value into a friendly label.
///
/// 3DMigoto key values can carry modifier prefixes (e.g. `no_modifiers VK_F`,
/// `ctrl alt VK_DELETE`) and either a `VK_*` virtual-key name or a bare
/// character. This translates the recognized `VK_*` token to a readable symbol
/// and preserves modifier words as a readable prefix.
pub fn translate_key(raw: &str) -> String {
    let tokens: Vec<&str> = raw.split_whitespace().collect();
    if tokens.is_empty() {
        return raw.to_string();
    }

    let mut modifiers: Vec<String> = Vec::new();
    let mut main: Option<String> = None;

    for tok in &tokens {
        let lower = tok.to_ascii_lowercase();
        // 3DMigoto negation tokens ("this modifier must NOT be held") carry no
        // meaning for display — drop them. `no_modifiers` is the "clear all"
        // form; `no_ctrl`/`no_shift`/`no_alt` negate a single modifier.
        if lower.starts_with("no_") {
            continue;
        }
        match lower.as_str() {
            "ctrl" | "control" => modifiers.push("Ctrl".to_string()),
            "alt" => modifiers.push("Alt".to_string()),
            "shift" => modifiers.push("Shift".to_string()),
            _ => {
                // First non-modifier token is the main key.
                if main.is_none() {
                    main = Some(translate_single_key(tok));
                }
            }
        }
    }

    let main = main.unwrap_or_else(|| raw.to_string());
    if modifiers.is_empty() {
        main
    } else {
        format!("{} + {}", modifiers.join(" + "), main)
    }
}

/// Translate a single key token (a `VK_*` name or a bare char) to a label.
fn translate_single_key(tok: &str) -> String {
    let upper = tok.to_ascii_uppercase();
    let label = match upper.as_str() {
        "VK_UP" => "↑ Arrow",
        "VK_DOWN" => "↓ Arrow",
        "VK_LEFT" => "← Arrow",
        "VK_RIGHT" => "→ Arrow",
        "VK_RETURN" => "Enter",
        "VK_ESCAPE" => "Esc",
        "VK_SPACE" => "Space",
        "VK_TAB" => "Tab",
        "VK_BACK" => "Backspace",
        "VK_DELETE" => "Delete",
        "VK_INSERT" => "Insert",
        "VK_HOME" => "Home",
        "VK_END" => "End",
        "VK_PRIOR" => "Page Up",
        "VK_NEXT" => "Page Down",
        "VK_SHIFT" => "Shift",
        "VK_CONTROL" => "Ctrl",
        "VK_MENU" => "Alt",
        "VK_CAPITAL" => "Caps Lock",
        "VK_NUMLOCK" => "Num Lock",
        "VK_OEM_3" => "` (Backtick)",
        "VK_OEM_MINUS" => "- (Minus)",
        "VK_OEM_PLUS" => "= (Equals)",
        "VK_OEM_COMMA" => ", (Comma)",
        "VK_OEM_PERIOD" => ". (Period)",
        // Numpad
        "VK_NUMPAD0" => "Numpad 0",
        "VK_NUMPAD1" => "Numpad 1",
        "VK_NUMPAD2" => "Numpad 2",
        "VK_NUMPAD3" => "Numpad 3",
        "VK_NUMPAD4" => "Numpad 4",
        "VK_NUMPAD5" => "Numpad 5",
        "VK_NUMPAD6" => "Numpad 6",
        "VK_NUMPAD7" => "Numpad 7",
        "VK_NUMPAD8" => "Numpad 8",
        "VK_NUMPAD9" => "Numpad 9",
        _ => {
            // Function keys VK_F1..VK_F24
            if let Some(n) = upper.strip_prefix("VK_F") {
                if n.chars().all(|c| c.is_ascii_digit()) && !n.is_empty() {
                    return format!("F{}", n);
                }
            }
            // A bare single character (e.g. `H`) — 3DMigoto accepts these.
            if tok.chars().count() == 1 {
                return tok.to_ascii_uppercase();
            }
            // Unknown VK_* or token — show as-is, but strip the VK_ noise.
            return tok.strip_prefix("VK_").unwrap_or(tok).to_string();
        }
    };
    label.to_string()
}

// ---------------------------------------------------------------------------
// Mod-level API (Phase 13): scan a mod folder's ini files, batch hash edits
// ---------------------------------------------------------------------------

use std::path::PathBuf;

/// One ini file within a mod, parsed for display.
#[derive(Debug, Clone, Serialize)]
pub struct ModIniFile {
    /// Absolute path to the ini file (used to target hash edits).
    pub path: String,
    /// Path relative to the mod folder, for display (e.g. "mod.ini",
    /// "sub/extra.ini").
    pub relative_path: String,
    /// Extracted keybinds in this file.
    pub keybinds: Vec<Keybind>,
    /// Extracted hashes in this file.
    pub hashes: Vec<HashEntry>,
}

/// One staged hash edit from the frontend. `ini_path` + `line_index` locate
/// the exact `hash =` line to rewrite; `new_value` is the corrected hash the
/// user pasted. `line_index` comes straight from the `HashEntry` we handed the
/// frontend, so it's guaranteed to point at a `hash` pair on read.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct HashEdit {
    pub ini_path: String,
    pub line_index: usize,
    pub new_value: String,
}

/// Result of a batch hash-update: which files were written, and any per-edit
/// or per-file errors.
#[derive(Debug, Clone, Serialize, Default)]
pub struct HashUpdateResult {
    /// Number of files that were actually modified + backed up.
    pub files_written: u32,
    /// Total number of hash lines changed across all files.
    pub hashes_changed: u32,
    pub errors: Vec<String>,
}

const MAX_INI_SCAN_DEPTH: u32 = 8;

/// Recursively collect every `.ini` file in a mod folder, skipping
/// 3DMigoto's own loader inis (`d3dx.ini`, `d3dx_user.ini`) and hidden files.
fn find_ini_files(root: &Path) -> Vec<PathBuf> {
    fn walk(dir: &Path, depth: u32, out: &mut Vec<PathBuf>) {
        if depth > MAX_INI_SCAN_DEPTH {
            return;
        }
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            if name.starts_with('.') {
                continue;
            }
            if path.is_dir() {
                walk(&path, depth + 1, out);
            } else if is_mod_ini(&name) {
                out.push(path);
            }
        }
    }

    let mut out = Vec::new();
    walk(root, 0, &mut out);
    out.sort();
    out
}

/// Whether a filename is a mod ini we should surface — an `.ini` that isn't
/// 3DMigoto's own loader config.
fn is_mod_ini(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    if !lower.ends_with(".ini") {
        return false;
    }
    // 3DMigoto's own loader inis are not mod content — skip them.
    !matches!(lower.as_str(), "d3dx.ini" | "d3dx_user.ini")
}

/// Read every mod ini in `mod_path`, returning each file's keybinds + hashes.
/// Files that can't be read are skipped (a mod with an unreadable ini
/// shouldn't blank out the whole panel).
pub fn read_mod_inis(mod_path: &str) -> Result<Vec<ModIniFile>, String> {
    let root = Path::new(mod_path);
    if !root.exists() {
        return Err(format!("Mod folder does not exist: {}", mod_path));
    }

    let mut files = Vec::new();
    for ini_path in find_ini_files(root) {
        let Ok(text) = fs::read_to_string(&ini_path) else {
            continue; // unreadable (binary/permission) — skip, don't fail all
        };
        let contents = extract(&parse(&text));
        // Only include files that actually have something to show.
        if contents.keybinds.is_empty() && contents.hashes.is_empty() {
            continue;
        }
        let relative_path = ini_path
            .strip_prefix(root)
            .unwrap_or(&ini_path)
            .to_string_lossy()
            .replace('\\', "/");
        files.push(ModIniFile {
            path: ini_path.to_string_lossy().to_string(),
            relative_path,
            keybinds: contents.keybinds,
            hashes: contents.hashes,
        });
    }

    Ok(files)
}

/// Apply a batch of hash edits, grouped by ini file so each touched file is
/// read/rewritten once with a single backup (a fresh non-colliding `.bak`
/// name per write). Continues past per-file failures, collecting errors rather
/// than aborting the whole batch.
pub fn update_mod_hashes(edits: &[HashEdit]) -> HashUpdateResult {
    use std::collections::BTreeMap;

    let mut result = HashUpdateResult::default();

    // Group edits by file path.
    let mut by_file: BTreeMap<&str, Vec<LineEdit>> = BTreeMap::new();
    for e in edits {
        by_file.entry(e.ini_path.as_str()).or_default().push(LineEdit {
            line_index: e.line_index,
            new_value: e.new_value.clone(),
        });
    }

    for (path, line_edits) in by_file {
        let p = Path::new(path);
        // Snapshot the file text to count how many lines actually change.
        let before = fs::read_to_string(p).ok();
        match write_line_edits(p, &line_edits) {
            Ok(errs) => {
                result.errors.extend(errs);
                if let Some(before) = before {
                    if let Ok(after) = fs::read_to_string(p) {
                        if before != after {
                            result.files_written += 1;
                            let changed = before
                                .lines()
                                .zip(after.lines())
                                .filter(|(a, b)| a != b)
                                .count() as u32;
                            result.hashes_changed += changed;
                        }
                    }
                }
            }
            Err(e) => result.errors.push(format!("{}: {}", path, e)),
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    fn temp_dir() -> std::path::PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "mod-manager-ini-test-{}-{}",
            std::process::id(),
            n
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    const SAMPLE: &str = "\
; Furina mod by someone
[Constants]
global persist $swapvar = 0

[KeySwap]
condition = $active == 1
key = VK_DOWN
back = VK_UP
type = cycle
$swapvar = 0,1,2

[TextureOverrideFurinaBody]
hash = 1a2b3c4d
run = CommandListFurinaBody
";

    #[test]
    fn roundtrip_preserves_file_byte_for_byte() {
        let doc = parse(SAMPLE);
        assert_eq!(doc.to_text(), SAMPLE, "round-trip must be byte-identical");
    }

    #[test]
    fn roundtrip_without_trailing_newline() {
        let text = "[A]\nkey = 1";
        let doc = parse(text);
        assert_eq!(doc.to_text(), text);
        assert!(!doc.trailing_newline);
    }

    #[test]
    fn classifies_line_kinds() {
        let doc = parse(SAMPLE);
        // First line is a comment.
        assert!(matches!(doc.lines[0], IniLine::Comment { .. }));
        // Section header.
        assert!(matches!(&doc.lines[1], IniLine::Section { name, .. } if name == "Constants"));
        // A pair.
        assert!(matches!(&doc.lines[2], IniLine::Pair { key, value, .. }
            if key == "global persist $swapvar" && value == "0"));
        // A blank line exists between sections.
        assert!(doc.lines.iter().any(|l| matches!(l, IniLine::Blank { .. })));
    }

    #[test]
    fn comments_and_blanks_survive_an_edit() {
        // Find the hash pair line and change its value; comments/blanks must remain.
        let mut doc = parse(SAMPLE);
        let idx = doc
            .lines
            .iter()
            .position(|l| matches!(l, IniLine::Pair { key, .. } if key == "hash"))
            .unwrap();

        let errors = apply_edits(&mut doc, &[LineEdit { line_index: idx, new_value: "deadbeef".into() }]);
        assert!(errors.is_empty());

        let out = doc.to_text();
        assert!(out.contains("hash = deadbeef"), "value should be updated");
        assert!(out.contains("; Furina mod by someone"), "leading comment must survive");
        assert!(out.contains("type = cycle"), "other pairs untouched");
        // Exactly one line changed vs original.
        let changed: Vec<_> = SAMPLE.lines().zip(out.lines()).filter(|(a, b)| a != b).collect();
        assert_eq!(changed.len(), 1, "only the hash line should differ");
    }

    #[test]
    fn edit_preserves_indentation_and_spacing_around_equals() {
        let text = "[S]\n    hash   =    oldval\n";
        let mut doc = parse(text);
        apply_edits(&mut doc, &[LineEdit { line_index: 1, new_value: "newval".into() }]);
        // Leading indent and the spacing after '=' are preserved; the run of
        // spaces before '=' is part of the preserved prefix.
        assert_eq!(doc.to_text(), "[S]\n    hash   =    newval\n");
    }

    #[test]
    fn apply_edits_rejects_non_pair_and_out_of_range() {
        let mut doc = parse(SAMPLE);
        // Line 1 is the [Constants] section header — not a pair.
        let errors = apply_edits(&mut doc, &[
            LineEdit { line_index: 1, new_value: "x".into() },
            LineEdit { line_index: 9999, new_value: "y".into() },
        ]);
        assert_eq!(errors.len(), 2);
        // Doc unchanged.
        assert_eq!(doc.to_text(), SAMPLE);
    }

    #[test]
    fn write_line_edits_creates_backup_and_writes() {
        let dir = temp_dir();
        let ini = dir.join("mod.ini");
        fs::write(&ini, SAMPLE).unwrap();

        let doc = parse(SAMPLE);
        let idx = doc
            .lines
            .iter()
            .position(|l| matches!(l, IniLine::Pair { key, .. } if key == "hash"))
            .unwrap();

        let errors = write_line_edits(&ini, &[LineEdit { line_index: idx, new_value: "cafef00d".into() }])
            .expect("write should succeed");
        assert!(errors.is_empty());

        // Backup holds the original.
        let bak = dir.join("mod.ini.bak");
        assert!(bak.exists());
        assert_eq!(fs::read_to_string(&bak).unwrap(), SAMPLE);

        // File has the new value.
        let updated = fs::read_to_string(&ini).unwrap();
        assert!(updated.contains("hash = cafef00d"));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn write_line_edits_no_op_does_not_write_backup() {
        let dir = temp_dir();
        let ini = dir.join("mod.ini");
        fs::write(&ini, SAMPLE).unwrap();

        // Edit that sets the same value it already has → net no change.
        let doc = parse(SAMPLE);
        let idx = doc
            .lines
            .iter()
            .position(|l| matches!(l, IniLine::Pair { key, value, .. } if key == "hash" && value == "1a2b3c4d"))
            .unwrap();

        let errors = write_line_edits(&ini, &[LineEdit { line_index: idx, new_value: "1a2b3c4d".into() }])
            .expect("write should succeed");
        assert!(errors.is_empty());
        // No backup because nothing changed.
        assert!(!dir.join("mod.ini.bak").exists());

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn write_line_edits_does_not_clobber_an_existing_backup() {
        let dir = temp_dir();
        let ini = dir.join("mod.ini");
        fs::write(&ini, SAMPLE).unwrap();

        let doc = parse(SAMPLE);
        let idx = doc
            .lines
            .iter()
            .position(|l| matches!(l, IniLine::Pair { key, .. } if key == "hash"))
            .unwrap();

        // First edit → mod.ini.bak holds the original SAMPLE.
        write_line_edits(&ini, &[LineEdit { line_index: idx, new_value: "1111".into() }]).unwrap();
        assert_eq!(fs::read_to_string(dir.join("mod.ini.bak")).unwrap(), SAMPLE);

        // Second edit → must NOT overwrite mod.ini.bak; goes to .bak.2 holding
        // the state after the first edit.
        write_line_edits(&ini, &[LineEdit { line_index: idx, new_value: "2222".into() }]).unwrap();
        assert_eq!(
            fs::read_to_string(dir.join("mod.ini.bak")).unwrap(),
            SAMPLE,
            "the original backup must be preserved"
        );
        let bak2 = fs::read_to_string(dir.join("mod.ini.bak.2")).unwrap();
        assert!(bak2.contains("hash = 1111"), "second backup holds the post-first-edit state");

        // Third edit → .bak.3.
        write_line_edits(&ini, &[LineEdit { line_index: idx, new_value: "3333".into() }]).unwrap();
        assert!(dir.join("mod.ini.bak.3").exists());
        assert!(fs::read_to_string(&ini).unwrap().contains("hash = 3333"));

        fs::remove_dir_all(&dir).ok();
    }

    // ----------------------------------------------------------------------
    // Extraction + translation
    // ----------------------------------------------------------------------

    #[test]
    fn extract_pulls_keybind_fields() {
        let contents = extract(&parse(SAMPLE));
        assert_eq!(contents.keybinds.len(), 1);
        let kb = &contents.keybinds[0];
        assert_eq!(kb.section, "KeySwap");
        assert_eq!(kb.key.as_deref(), Some("VK_DOWN"));
        assert_eq!(kb.key_label.as_deref(), Some("↓ Arrow"));
        assert_eq!(kb.back.as_deref(), Some("VK_UP"));
        assert_eq!(kb.back_label.as_deref(), Some("↑ Arrow"));
        assert_eq!(kb.bind_type.as_deref(), Some("cycle"));
        assert_eq!(kb.variable.as_deref(), Some("$swapvar"));
        assert_eq!(kb.values.as_deref(), Some("0,1,2"));
    }

    #[test]
    fn extract_pulls_hash_with_section_and_line_index() {
        let doc = parse(SAMPLE);
        let contents = extract(&doc);
        assert_eq!(contents.hashes.len(), 1);
        let h = &contents.hashes[0];
        assert_eq!(h.section, "TextureOverrideFurinaBody");
        assert_eq!(h.value, "1a2b3c4d");
        // The recorded line index must actually point at the hash pair.
        assert!(matches!(&doc.lines[h.line_index],
            IniLine::Pair { key, value, .. } if key == "hash" && value == "1a2b3c4d"));
    }

    #[test]
    fn extract_handles_multiple_keys_and_hashes() {
        let text = "\
[KeyToggle]
key = VK_F
type = toggle

[KeySwapVariants]
key = h
back = j
type = cycle
$var = 0,1

[TextureOverrideA]
hash = aaaa

[TextureOverrideB]
hash = bbbb
";
        let contents = extract(&parse(text));
        assert_eq!(contents.keybinds.len(), 2);
        assert_eq!(contents.keybinds[0].section, "KeyToggle");
        assert_eq!(contents.keybinds[0].bind_type.as_deref(), Some("toggle"));
        assert_eq!(contents.keybinds[1].key_label.as_deref(), Some("H"));
        assert_eq!(contents.keybinds[1].back_label.as_deref(), Some("J"));

        assert_eq!(contents.hashes.len(), 2);
        assert_eq!(contents.hashes[0].section, "TextureOverrideA");
        assert_eq!(contents.hashes[1].section, "TextureOverrideB");
        assert_ne!(contents.hashes[0].line_index, contents.hashes[1].line_index);
    }

    #[test]
    fn extract_ignores_empty_key_sections() {
        // A [Key*] section with no key/back/type bindings shouldn't produce a
        // keybind entry.
        let text = "[KeyContextThing]\ncondition = $active == 1\n";
        let contents = extract(&parse(text));
        assert!(contents.keybinds.is_empty());
    }

    #[test]
    fn translate_key_maps_common_vk_names() {
        assert_eq!(translate_key("VK_DOWN"), "↓ Arrow");
        assert_eq!(translate_key("VK_RETURN"), "Enter");
        assert_eq!(translate_key("VK_F10"), "F10");
        assert_eq!(translate_key("VK_NUMPAD5"), "Numpad 5");
    }

    #[test]
    fn translate_key_handles_modifiers_and_bare_chars() {
        assert_eq!(translate_key("no_modifiers VK_F"), "F");
        assert_eq!(translate_key("ctrl alt VK_DELETE"), "Ctrl + Alt + Delete");
        assert_eq!(translate_key("h"), "H");
        assert_eq!(translate_key("shift VK_UP"), "Shift + ↑ Arrow");
    }

    #[test]
    fn translate_key_drops_per_modifier_negations() {
        // `no_ctrl`/`no_shift`/`no_alt` mean "must NOT be held" — they should
        // be dropped, and must NOT swallow the real key. Regression for
        // "ctrl no_shift no_alt VK_UP" rendering as just "Ctrl + Shift".
        assert_eq!(translate_key("ctrl no_shift no_alt VK_UP"), "Ctrl + ↑ Arrow");
        assert_eq!(translate_key("no_ctrl no_shift no_alt VK_F"), "F");
        assert_eq!(translate_key("no_alt shift VK_DOWN"), "Shift + ↓ Arrow");
    }

    #[test]
    fn translate_key_falls_back_gracefully_on_unknown() {
        // Unknown VK_* strips the VK_ prefix; empty stays empty-ish.
        assert_eq!(translate_key("VK_SOMETHING_NEW"), "SOMETHING_NEW");
    }

    // ----------------------------------------------------------------------
    // Mod-level: read_mod_inis + update_mod_hashes
    // ----------------------------------------------------------------------

    fn write_file(path: &Path, content: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    #[test]
    fn read_mod_inis_finds_files_skips_loader_and_empty() {
        let mod_dir = temp_dir();
        // A real mod ini with content.
        write_file(&mod_dir.join("mod.ini"), SAMPLE);
        // A nested ini with only a hash.
        write_file(&mod_dir.join("sub/extra.ini"), "[TextureOverrideX]\nhash = ffff\n");
        // 3DMigoto's loader ini — must be skipped.
        write_file(&mod_dir.join("d3dx_user.ini"), "$\\mods\\x\\ = 1\n");
        // An ini with nothing interesting — must be skipped.
        write_file(&mod_dir.join("notes.ini"), "; just a note\n[Info]\nauthor = someone\n");
        // A hidden file — skipped.
        write_file(&mod_dir.join(".hidden.ini"), "[TextureOverrideZ]\nhash = 9999\n");

        let files = read_mod_inis(mod_dir.to_str().unwrap()).expect("read should succeed");
        let names: Vec<&str> = files.iter().map(|f| f.relative_path.as_str()).collect();

        assert!(names.contains(&"mod.ini"), "got {:?}", names);
        assert!(names.contains(&"sub/extra.ini"), "got {:?}", names);
        assert!(!names.iter().any(|n| n.contains("d3dx")), "loader ini must be skipped");
        assert!(!names.contains(&"notes.ini"), "content-free ini must be skipped");
        assert!(!names.iter().any(|n| n.contains("hidden")), "hidden ini must be skipped");

        // mod.ini carries a keybind + a hash.
        let modini = files.iter().find(|f| f.relative_path == "mod.ini").unwrap();
        assert_eq!(modini.keybinds.len(), 1);
        assert_eq!(modini.hashes.len(), 1);

        fs::remove_dir_all(&mod_dir).ok();
    }

    #[test]
    fn read_mod_inis_errors_on_missing_folder() {
        assert!(read_mod_inis("/definitely/not/here/mod-manager-ini").is_err());
    }

    #[test]
    fn update_mod_hashes_writes_targeted_edits_with_backup() {
        let mod_dir = temp_dir();
        let ini = mod_dir.join("mod.ini");
        write_file(&ini, SAMPLE);

        // Read to get the real line index of the hash.
        let files = read_mod_inis(mod_dir.to_str().unwrap()).unwrap();
        let hash_entry = &files.iter().find(|f| f.relative_path == "mod.ini").unwrap().hashes[0];

        let result = update_mod_hashes(&[HashEdit {
            ini_path: ini.to_string_lossy().to_string(),
            line_index: hash_entry.line_index,
            new_value: "feedface".to_string(),
        }]);

        assert_eq!(result.files_written, 1);
        assert_eq!(result.hashes_changed, 1);
        assert!(result.errors.is_empty());

        // File updated, backup present with original.
        assert!(fs::read_to_string(&ini).unwrap().contains("hash = feedface"));
        assert_eq!(fs::read_to_string(mod_dir.join("mod.ini.bak")).unwrap(), SAMPLE);

        fs::remove_dir_all(&mod_dir).ok();
    }

    #[test]
    fn update_mod_hashes_groups_multiple_files_one_backup_each() {
        let mod_dir = temp_dir();
        let a = mod_dir.join("a.ini");
        let b = mod_dir.join("b.ini");
        write_file(&a, "[TextureOverrideA]\nhash = aaaa\n");
        write_file(&b, "[TextureOverrideB]\nhash = bbbb\n");

        let result = update_mod_hashes(&[
            HashEdit { ini_path: a.to_string_lossy().to_string(), line_index: 1, new_value: "1111".into() },
            HashEdit { ini_path: b.to_string_lossy().to_string(), line_index: 1, new_value: "2222".into() },
        ]);

        assert_eq!(result.files_written, 2);
        assert_eq!(result.hashes_changed, 2);
        assert!(fs::read_to_string(&a).unwrap().contains("hash = 1111"));
        assert!(fs::read_to_string(&b).unwrap().contains("hash = 2222"));
        assert!(mod_dir.join("a.ini.bak").exists());
        assert!(mod_dir.join("b.ini.bak").exists());

        fs::remove_dir_all(&mod_dir).ok();
    }
}
