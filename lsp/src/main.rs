use regex::Regex;
use serde_json::{json, Map, Value};
use std::{collections::HashMap, fs, io::{self, BufRead, Write}, path::{Path, PathBuf}};

const SKIP: &[&str] = &[".git", ".idea", ".zed", ".gradle", "node_modules", "target", "build"];
#[derive(Clone, Copy)] struct Span { start: usize, end: usize }
struct Doc { path: PathBuf, text: String, entries: HashMap<String, Span> }
struct Index { root: PathBuf, xml: HashMap<String, Doc>, java: HashMap<String, Doc>, mapper: Regex, stmt: Regex, package: Regex, interface: Regex, method: Regex, ident: Regex }

impl Index {
    fn new(root: PathBuf) -> Self { Self { root, xml: HashMap::new(), java: HashMap::new(), mapper: Regex::new(r#"(?is)<mapper\b[^>]*\bnamespace\s*=\s*[\"']([^\"']+)[\"']"#).unwrap(), stmt: Regex::new(r#"(?is)<(select|insert|update|delete)\b[^>]*\bid\s*=\s*[\"']([^\"']+)[\"'][^>]*>"#).unwrap(), package: Regex::new(r"(?m)^\s*package\s+([\w.]+)\s*;").unwrap(), interface: Regex::new(r"\b(?:public\s+)?interface\s+(\w+)").unwrap(), method: Regex::new(r"(?m)^\s*(?:public\s+|protected\s+|private\s+)?(?:static\s+)?(?:default\s+)?[\w<>\[\], ?.@]+\s+(\w+)\s*\([^;{}]*\)\s*;").unwrap(), ident: Regex::new(r"[A-Za-z_$][\w$]*").unwrap() } }
    fn rebuild(&mut self) {
        self.xml.clear(); self.java.clear(); let mut paths = Vec::new(); walk(&self.root, &mut paths);
        for path in &paths { if ext(path, "xml") { self.xml_file(path); } }
        for path in &paths { if ext(path, "java") { self.java_file(path); } }
    }
    fn xml_file(&mut self, path: &Path) {
        let Ok(text) = fs::read_to_string(path) else { return }; let Some(c) = self.mapper.captures(&text) else { return };
        let namespace = c.get(1).unwrap().as_str().to_string();
        let entries = self.stmt.captures_iter(&text).filter_map(|c| { let id = c.get(2)?; let full = c.get(0)?; Some((id.as_str().to_string(), Span { start: full.start(), end: full.end() })) }).collect();
        self.xml.insert(namespace, Doc { path: path.to_path_buf(), text, entries });
    }
    fn java_file(&mut self, path: &Path) {
        let Ok(text) = fs::read_to_string(path) else { return }; let (Some(p), Some(i)) = (self.package.captures(&text), self.interface.captures(&text)) else { return };
        let fqn = format!("{}.{}", p.get(1).unwrap().as_str(), i.get(1).unwrap().as_str());
        let entries = self.method.captures_iter(&text).filter_map(|c| { let method = c.get(1)?; Some((method.as_str().to_string(), Span { start: method.start(), end: method.end() })) }).collect();
        self.java.insert(fqn, Doc { path: path.to_path_buf(), text, entries });
    }
    fn definition(&self, path: &Path, position: &Value) -> Option<Value> {
        let text = fs::read_to_string(path).ok()?; let off = offset(&text, position)?;
        if ext(path, "java") {
            let (Some(p), Some(i)) = (self.package.captures(&text), self.interface.captures(&text)) else { return None };
            let xml = self.xml.get(&format!("{}.{}", p.get(1)?.as_str(), i.get(1)?.as_str()))?;
            if let Some(word) = self.ident.find_iter(&text).find(|x| inside(x.start(), x.end(), off)).map(|x| x.as_str()) { if let Some(span) = xml.entries.get(word) { return Some(loc(xml, *span)); } }
            let name = i.get(1)?; if inside(name.start(), name.end(), off) { let namespace = self.mapper.captures(&xml.text)?.get(1)?; return Some(loc(xml, Span { start: namespace.start(), end: namespace.end() })); }
        } else if ext(path, "xml") {
            let mapper = self.mapper.captures(&text)?; let java = self.java.get(mapper.get(1)?.as_str())?; let namespace = mapper.get(1)?;
            if inside(namespace.start(), namespace.end(), off) { let name = self.interface.captures(&java.text)?.get(1)?; return Some(loc(java, Span { start: name.start(), end: name.end() })); }
            for tag in self.stmt.captures_iter(&text) { let full = tag.get(0)?; if inside(full.start(), full.end(), off) { if let Some(span) = java.entries.get(tag.get(2)?.as_str()) { return Some(loc(java, *span)); } } }
        }
        None
    }
    fn lenses(&self, path: &Path) -> Vec<Value> {
        let Ok(text) = fs::read_to_string(path) else { return vec![] };
        if ext(path, "java") {
            let (Some(p), Some(i)) = (self.package.captures(&text), self.interface.captures(&text)) else { return vec![] }; let Some(xml) = self.xml.get(&format!("{}.{}", p.get(1).unwrap().as_str(), i.get(1).unwrap().as_str())) else { return vec![] };
            self.method.captures_iter(&text).filter_map(|c| { let m = c.get(1)?; Some(lens(&text, m.start(), &format!("↗ XML: {}", m.as_str()), xml, *xml.entries.get(m.as_str())?)) }).collect()
        } else if ext(path, "xml") {
            let Some(mapper) = self.mapper.captures(&text) else { return vec![] }; let Some(java) = self.java.get(mapper.get(1).unwrap().as_str()) else { return vec![] };
            self.stmt.captures_iter(&text).filter_map(|c| { let tag = c.get(0)?; let m = c.get(2)?; Some(lens(&text, tag.start(), &format!("↗ Mapper: {}", m.as_str()), java, *java.entries.get(m.as_str())?)) }).collect()
        } else { vec![] }
    }
}
fn ext(path: &Path, expected: &str) -> bool { path.extension().and_then(|x| x.to_str()) == Some(expected) }
fn walk(root: &Path, out: &mut Vec<PathBuf>) { let Ok(entries) = fs::read_dir(root) else { return }; for item in entries.flatten() { let path = item.path(); if path.is_dir() { if !SKIP.iter().any(|x| path.file_name().and_then(|n| n.to_str()) == Some(*x)) { walk(&path, out); } } else if ext(&path, "java") || ext(&path, "xml") { out.push(path); } } }
fn inside(start: usize, end: usize, offset: usize) -> bool { start <= offset && offset <= end }
fn pos(text: &str, offset: usize) -> Value { let offset = offset.min(text.len()); let before = &text[..offset]; let line = before.bytes().filter(|x| *x == b'\n').count(); let start = before.rfind('\n').map(|x| x + 1).unwrap_or(0); json!({"line": line, "character": text[start..offset].encode_utf16().count()}) }
fn loc(doc: &Doc, span: Span) -> Value { json!({"uri": uri(&doc.path), "range": {"start": pos(&doc.text, span.start), "end": pos(&doc.text, span.end)}}) }
fn lens(source: &str, offset: usize, title: &str, target: &Doc, span: Span) -> Value { let p = pos(&target.text, span.start); json!({"range": {"start": pos(source, offset), "end": pos(source, offset)}, "command": {"title": title, "command": "mybatis-navigation.open", "arguments": [{"uri": uri(&target.path), "position": p}]}}) }
fn uri(path: &Path) -> String { let mut result = String::from("file://"); for b in path.to_string_lossy().bytes() { if b.is_ascii_alphanumeric() || matches!(b, b'/'|b'-'|b'_'|b'.'|b'~'|b':') { result.push(b as char) } else { result.push_str(&format!("%{b:02X}")) } } result }
fn path(uri: &str) -> Option<PathBuf> { let raw = uri.strip_prefix("file://")?.as_bytes(); let mut bytes = Vec::new(); let mut i = 0; while i < raw.len() { if raw[i] == b'%' && i + 2 < raw.len() { bytes.push(u8::from_str_radix(std::str::from_utf8(&raw[i + 1..i + 3]).ok()?, 16).ok()?); i += 3 } else { bytes.push(raw[i]); i += 1 } } String::from_utf8(bytes).ok().map(PathBuf::from) }
fn offset(text: &str, p: &Value) -> Option<usize> { let line = p.get("line")?.as_u64()? as usize; let char = p.get("character")?.as_u64()? as usize; let start = if line == 0 { 0 } else { text.match_indices('\n').nth(line - 1).map(|(i, _)| i + 1)? }; let end = text[start..].find('\n').map(|x| start + x).unwrap_or(text.len()); let mut width = 0; for (i, c) in text[start..end].char_indices() { if width >= char { return Some(start + i) } width += c.len_utf16(); if width >= char { return Some(start + i + c.len_utf8()) } } Some(end) }
fn read(reader: &mut impl BufRead) -> io::Result<Option<Value>> { let mut len = None; loop { let mut line = String::new(); if reader.read_line(&mut line)? == 0 { return Ok(None) } if line == "\r\n" || line == "\n" { break } if let Some((k, v)) = line.split_once(':') { if k.eq_ignore_ascii_case("content-length") { len = v.trim().parse().ok() } } } let Some(len) = len else { return Err(io::Error::new(io::ErrorKind::InvalidData, "Content-Length missing")) }; let mut body = vec![0; len]; reader.read_exact(&mut body)?; serde_json::from_slice(&body).map(Some).map_err(io::Error::other) }
fn send(writer: &mut impl Write, value: Value) -> io::Result<()> { let bytes = serde_json::to_vec(&value).map_err(io::Error::other)?; write!(writer, "Content-Length: {}\r\n\r\n", bytes.len())?; writer.write_all(&bytes)?; writer.flush() }
fn main() -> io::Result<()> {
    let stdin = io::stdin(); let stdout = io::stdout(); let mut reader = stdin.lock(); let mut writer = stdout.lock(); let mut index: Option<Index> = None;
    while let Some(message) = read(&mut reader)? { let method = message.get("method").and_then(Value::as_str).unwrap_or(""); let id = message.get("id").cloned(); let params = message.get("params").cloned().unwrap_or_else(|| Value::Object(Map::new())); match method {
        "initialize" => { let root = params.get("workspaceFolders").and_then(Value::as_array).and_then(|x| x.first()).and_then(|x| x.get("uri")).and_then(Value::as_str).or_else(|| params.get("rootUri").and_then(Value::as_str)).and_then(path); if let Some(root) = root { let mut next = Index::new(root); next.rebuild(); index = Some(next); } if let Some(id) = id { send(&mut writer, json!({"jsonrpc":"2.0","id":id,"result":{"capabilities":{"textDocumentSync":1,"definitionProvider":true,"codeLensProvider":{"resolveProvider":false},"executeCommandProvider":{"commands":["mybatis-navigation.open"]}},"serverInfo":{"name":"MyBatis Navigation","version":"0.2.0"}}}))?; } }
        "textDocument/definition" => { let result = index.as_ref().and_then(|_| params.get("textDocument")?.get("uri")?.as_str()).and_then(path).and_then(|p| index.as_ref()?.definition(&p, params.get("position").unwrap_or(&Value::Null))); if let Some(id) = id { send(&mut writer, json!({"jsonrpc":"2.0","id":id,"result":result}))?; } }
        "textDocument/codeLens" => { let result = index.as_ref().and_then(|_| params.get("textDocument")?.get("uri")?.as_str()).and_then(path).map(|p| index.as_ref().unwrap().lenses(&p)).unwrap_or_default(); if let Some(id) = id { send(&mut writer, json!({"jsonrpc":"2.0","id":id,"result":result}))?; } }
        "workspace/executeCommand" => { if let Some(id) = id { send(&mut writer, json!({"jsonrpc":"2.0","id":id,"result":null}))?; } if params.get("command").and_then(Value::as_str) == Some("mybatis-navigation.open") { if let Some(target) = params.get("arguments").and_then(Value::as_array).and_then(|x| x.first()) { send(&mut writer, json!({"jsonrpc":"2.0","id":"mybatis-navigation-show-document","method":"window/showDocument","params":{"uri":target.get("uri"),"selection":{"start":target.get("position"),"end":target.get("position")},"takeFocus":true}}))?; } } }
        "textDocument/didSave" | "workspace/didChangeWatchedFiles" => if let Some(x) = index.as_mut() { x.rebuild(); },
        "shutdown" => if let Some(id) = id { send(&mut writer, json!({"jsonrpc":"2.0","id":id,"result":null}))?; }, "exit" => return Ok(()), _ => {} }
    } Ok(())
}
