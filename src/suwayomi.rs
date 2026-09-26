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
