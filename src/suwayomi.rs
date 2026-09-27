use aidoku::{
    alloc::{String, Vec, format},
    imports::net::Request,
    prelude::*,
    Result,
};
use serde::Deserialize;

#[derive(Deserialize)]
struct Gql<T> { data: T }

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SmManga { pub id: i32, pub title: String, pub thumbnail_url: Option<String>,
    pub description: Option<String>, pub author: Option<String>, pub artist: Option<String>,
    pub genre: Option<Vec<String>>, pub status: Option<String>, pub real_url: Option<String>, pub url: Option<String> }
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SmList { pub fetch_source_manga: SmListPayload }
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SmListPayload { pub mangas: Vec<SmManga>, pub has_next_page: Option<bool> }
#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SmCh { pub id: i32, pub name: String, pub chapter_number: Option<f32>,
    pub upload_date: Option<String>, pub scanlator: Option<String>, pub real_url: Option<String> }
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SmChs { pub fetch_chapters: SmChsPayload }
#[derive(Deserialize)]
pub struct SmChsPayload { pub chapters: Vec<SmCh> }
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SmPages { pub fetch_chapter_pages: SmPagesPayload }
#[derive(Deserialize)]
pub struct SmPagesPayload { pub pages: Vec<String> }
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddByUrl { pub add_manga_from_url: AddByUrlPayload }
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddByUrlPayload { pub status: Option<String>, pub message: Option<String>,
    pub installed_extension_pkg_name: Option<String>,
    pub manga: Option<SmManga> }

fn b64(input: &[u8]) -> String {
    const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut o = String::new();
    let mut i = 0;
    while i < input.len() {
        let b0 = input[i] as u32;
        let b1 = if i+1 < input.len() { input[i+1] as u32 } else { 0 };
        let b2 = if i+2 < input.len() { input[i+2] as u32 } else { 0 };
        let n = (b0<<16)|(b1<<8)|b2;
        o.push(T[((n>>18)&0x3f) as usize] as char);
        o.push(T[((n>>12)&0x3f) as usize] as char);
        o.push(if i+1 < input.len() { T[((n>>6)&0x3f) as usize] as char } else { '=' });
        o.push(if i+2 < input.len() { T[(n&0x3f) as usize] as char } else { '=' });
        i += 3;
    }
    o
}

pub fn auth_val(user: &Option<String>, pass: &Option<String>) -> Option<String> {
    let u = user.as_deref().filter(|s| !s.is_empty())?;
    Some(format!("Basic {}", b64(format!("{}:{}", u, pass.as_deref().unwrap_or("")).as_bytes())))
}

fn b64val(c: u8) -> Option<u32> {
    match c {
        b'A'..=b'Z' => Some((c - b'A') as u32),
        b'a'..=b'z' => Some((c - b'a' + 26) as u32),
        b'0'..=b'9' => Some((c - b'0' + 52) as u32),
        b'+' => Some(62), b'/' => Some(63),
        _ => None,
    }
}

fn b64decode(s: &str) -> Option<Vec<u8>> {
    let b = s.as_bytes();
    if b.is_empty() || b.len() % 4 != 0 { return None; }
    let mut o = Vec::new();
    let mut i = 0;
    while i < b.len() {
        let mut n: u32 = 0;
        let mut pad = 0;
        for j in 0..4 {
            let c = b[i + j];
            if c == b'=' {
                if i + j < b.len() - 2 { return None; }
                pad += 1; n <<= 6;
            } else {
                if pad > 0 { return None; }
                n = (n << 6) | b64val(c)?;
            }
        }
        if pad > 2 { return None; }
        o.push(((n >> 16) & 0xff) as u8);
        if pad < 2 { o.push(((n >> 8) & 0xff) as u8); }
        if pad < 1 { o.push((n & 0xff) as u8); }
        i += 4;
    }
    Some(o)
}

pub fn deobf(raw: &str, base: &str, user: &Option<String>) -> Option<String> {
    let body = raw.strip_prefix("obf1:")?;
    let (nonce, data_b64) = body.split_once('.')?;
    if nonce.is_empty() || nonce.len() > 32 || !nonce.bytes().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let data = b64decode(data_b64)?;
    let kb = sha256(format!("{}:{}:{}:taisendev-obf1", nonce, base, user.as_deref().unwrap_or("")).as_bytes());
    if data.is_empty() { return None; }
    let clr: Vec<u8> = data.iter().enumerate().map(|(i, &x)| x ^ kb[i % kb.len()]).collect();
    core::str::from_utf8(&clr).ok().map(String::from)
}

fn sha256(msg: &[u8]) -> [u8; 32] {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
        0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
        0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
        0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
        0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
        0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
        0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
        0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
    ];
    let mut padded = Vec::new();
    padded.extend_from_slice(msg);
    padded.push(0x80);
    while padded.len() % 64 != 56 { padded.push(0); }
    padded.extend_from_slice(&((msg.len() as u64).wrapping_mul(8)).to_be_bytes());
    for chunk in padded.chunks_exact(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([chunk[4 * i], chunk[4 * i + 1], chunk[4 * i + 2], chunk[4 * i + 3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
        }
        let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh) =
            (h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7]);
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let t1 = hh.wrapping_add(s1).wrapping_add(ch).wrapping_add(K[i]).wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            hh = g; g = f; f = e; e = d.wrapping_add(t1); d = c; c = b; b = a; a = t1.wrapping_add(t2);
        }
        h[0] = h[0].wrapping_add(a); h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c); h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e); h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g); h[7] = h[7].wrapping_add(hh);
    }
    let mut out = [0u8; 32];
    for i in 0..8 { out[4 * i..4 * i + 4].copy_from_slice(&h[i].to_be_bytes()); }
    out
}

fn gql<T: serde::de::DeserializeOwned>(base: &str, user: &Option<String>, pass: &Option<String>, query: &str) -> Result<T> {
    #[derive(serde::Serialize)]
    struct Body<'a> { query: &'a str }
    let url = format!("{}/api/graphql", base);
    let body = serde_json::to_vec(&Body { query }).map_err(|e| error!("json: {:?}", e))?;
    let mut req = Request::post(&url).map_err(|e| error!("req: {:?}", e))?;
    req.set_header("Content-Type", "application/json");
    req.set_header("Accept", "application/json");
    if let Some(a) = auth_val(user, pass) { req.set_header("Authorization", &a); }
    req.set_body(&body);
    let g: Gql<T> = req.send().map_err(|e| error!("send: {:?}", e))?
        .get_json_owned().map_err(|e| error!("parse: {:?}", e))?;
    Ok(g.data)
}

pub fn esc(s: &str) -> String {
    let mut o = String::from(s);
    o = o.replace('\\', "");
    o = o.replace('"', "");
    o
}

pub fn fetch_source(base: &str, u: &Option<String>, p: &Option<String>, src: &str, ty: &str, q: Option<&str>, page: i32, filters: &[String]) -> Result<SmListPayload> {
    let qs = q.map(|s| format!(r#"query: "{}""#, esc(s))).unwrap_or_default();
    let fs = if filters.is_empty() { String::new() } else { format!(r#",filters: [{}]"#, filters.join(",")) };
    let query = format!(r#"mutation {{ fetchSourceManga(input: {{ source: "{src}", type: {ty}, {qs} page: {page}{fs} }}) {{ mangas {{ id title thumbnailUrl description author artist genre status realUrl url }} hasNextPage }} }}"#);
    gql::<SmList>(base, u, p, &query).map(|d| d.fetch_source_manga)
}

pub fn fetch_chapters(base: &str, u: &Option<String>, p: &Option<String>, manga_id: i32) -> Result<Vec<SmCh>> {
    let q = format!(r#"mutation {{ fetchChapters(input: {{ mangaId: {manga_id} }}) {{ chapters {{ id name chapterNumber uploadDate scanlator realUrl }} }} }}"#);
    gql::<SmChs>(base, u, p, &q).map(|d| d.fetch_chapters.chapters)
}

pub fn fetch_pages(base: &str, u: &Option<String>, p: &Option<String>, chapter_id: i32) -> Result<Vec<String>> {
    let q = format!(r#"mutation {{ fetchChapterPages(input: {{ chapterId: {chapter_id} }}) {{ pages }} }}"#);
    gql::<SmPages>(base, u, p, &q).map(|d| d.fetch_chapter_pages.pages)
}

pub fn add_by_url(base: &str, u: &Option<String>, p: &Option<String>, url: &str) -> Result<AddByUrlPayload> {
    let q = format!(r#"mutation {{ addMangaFromUrl(input: {{ url: "{}", autoInstallExtension: true, addToLibrary: true }}) {{ status message installedExtensionPkgName manga {{ id title url }} }} }}"#, esc(url));
    gql::<AddByUrl>(base, u, p, &q).map(|d| d.add_manga_from_url)
}

pub fn abs(base: &str, path: &str) -> String {
    if path.starts_with("http://") || path.starts_with("https://") { String::from(path) }
    else { format!("{base}{path}") }
}

pub fn manga_key(src: &str, m: &SmManga) -> String {
    let url = m.real_url.clone().or(m.url.clone()).unwrap_or_default();
    if url.starts_with("http://") || url.starts_with("https://") {
        format!("sm|{src}|{}|{url}", m.id)
    } else {
        format!("{}", m.id)
    }
}

fn split_key(key: &str) -> Option<(&str, i32, &str)> {
    let rest = key.strip_prefix("sm|")?;
    let (s, rest) = rest.split_once('|')?;
    let (id_s, url) = rest.split_once('|')?;
    if s.is_empty() || url.is_empty() { return None; }
    Some((s, id_s.parse::<i32>().ok()?, url))
}

pub fn resolve_manga_id(base: &str, u: &Option<String>, p: &Option<String>, src: &str, key: &str) -> Result<i32> {
    if let Ok(id) = key.parse::<i32>() { return Ok(id); }
    let (s, row_id, url) = split_key(key).ok_or(error!("bad manga key"))?;
    if s != src { return Err(error!("manga key mismatch")); }
    if let Ok(d) = fetch_manga_details(base, u, p, row_id) {
        let live = d.real_url.as_deref().or(d.url.as_deref()).unwrap_or("");
        if live == url { return Ok(row_id); }
    }
    let r = add_by_url(base, u, p, url)?;
    r.manga.map(|m| m.id).ok_or(error!("could not resolve manga"))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SmFull { pub fetch_manga_and_chapters: SmFullPayload }
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SmFullPayload { pub manga: SmManga }

pub fn fetch_manga_details(base: &str, u: &Option<String>, p: &Option<String>, manga_id: i32) -> Result<SmManga> {
    let q = format!(r#"mutation {{ fetchMangaAndChapters(input: {{ id: {manga_id}, fetchManga: true, fetchChapters: false }}) {{ manga {{ id title thumbnailUrl description author artist genre status realUrl url }} }} }}"#);
    gql::<SmFull>(base, u, p, &q).map(|d| d.fetch_manga_and_chapters.manga)
}

#[derive(Deserialize)]
#[serde(tag = "__typename")]
pub enum SwFilter {
    CheckBoxFilter { name: String, #[serde(default)] cbDefault: bool },
    SelectFilter { name: String, #[serde(default)] selDefault: i32, #[serde(default)] values: Vec<String> },
    TextFilter { name: String },
    #[serde(other)] Other,
}

#[derive(Deserialize)]
pub struct SwFilters { pub source: SwFiltersPayload }
#[derive(Deserialize)]
pub struct SwFiltersPayload { pub filters: Vec<SwFilter> }

pub fn fetch_filters(base: &str, u: &Option<String>, p: &Option<String>, src: &str) -> Result<Vec<SwFilter>> {
    let q = format!(r#"query {{ source(id: "{src}") {{ filters {{ __typename ... on CheckBoxFilter {{ name cbDefault: default }} ... on SelectFilter {{ name selDefault: default values }} ... on TextFilter {{ name }} }} }} }}"#);
    gql::<SwFilters>(base, u, p, &q).map(|d| d.source.filters)
}
