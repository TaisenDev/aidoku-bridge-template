#![no_std]
use aidoku::{alloc::{String, Vec, format, borrow::Cow}, imports::{defaults::defaults_get, net::Request},
    prelude::*, Chapter, Filter, FilterKind, FilterValue, Listing, ListingProvider, Manga, MangaPageResult, MangaStatus,
    Page, PageContent, Result, Source, DynamicFilters, ImageRequestProvider};

mod suwayomi;
use suwayomi::SmManga;

struct Bridge;

impl Bridge {
    fn cfg(&self) -> Result<(String, Option<String>, Option<String>, String)> {
        let base = defaults_get::<String>("serverUrl").filter(|s| !s.is_empty())
            .ok_or(aidoku::AidokuError::Message(String::from("Set the Suwayomi server URL in source settings")))?;
        let u = defaults_get::<String>("username").filter(|s| !s.is_empty());
        let p = defaults_get::<String>("password").filter(|s| !s.is_empty());
        let src = defaults_get::<String>("sourceId").filter(|s| !s.is_empty())
            .ok_or(aidoku::AidokuError::Message(String::from("Set the Suwayomi source ID in source settings")))?;
        Ok((base, u, p, src))
    }
    fn manga(m: &SmManga, base: &str) -> Manga {
        Manga { key: format!("{}", m.id), title: m.title.clone(),
            cover: Some(format!("{base}/api/v1/manga/{}/thumbnail", m.id)),
            authors: m.author.clone().filter(|s| !s.is_empty()).map(|a| Vec::from([a])),
            artists: m.artist.clone().filter(|s| !s.is_empty()).map(|a| Vec::from([a])),
            description: m.description.clone(), url: m.real_url.clone().or(m.url.clone()),
            tags: m.genre.clone(), status: match m.status.as_deref() {
                Some("ONGOING") => MangaStatus::Ongoing, Some("COMPLETED") => MangaStatus::Completed,
                Some("HIATUS") => MangaStatus::Hiatus, Some("CANCELLED") => MangaStatus::Cancelled,
                _ => MangaStatus::Unknown },
            ..Default::default() }
    }
}

fn to_change(pos: usize, fv: &FilterValue, defs: &[suwayomi::SwFilter]) -> Option<String> {
    match fv {
        FilterValue::Check { value, .. } => {
            Some(format!(r#"{{position:{pos},checkBoxState:{}}}"#, *value != 0))
        }
        FilterValue::Text { value, .. } => {
            Some(format!(r#"{{position:{pos},textState:"{}"}}"#, suwayomi::esc(value)))
        }
        FilterValue::Select { value, .. } => {
            let idx = match defs.get(pos) {
                Some(suwayomi::SwFilter::SelectFilter { values, .. }) => {
                    values.iter().position(|o| o == value).or_else(|| {
                        value.parse::<i32>().ok().filter(|&n| n >= 0 && (n as usize) < values.len()).map(|n| n as usize)
                    })
                }
                _ => None,
            };
            idx.map(|n| format!(r#"{{position:{pos},selectState:{n}}}"#))
        }
        _ => None,
    }
}

impl DynamicFilters for Bridge {
    fn get_dynamic_filters(&self) -> Result<Vec<Filter>> {
        let (base, u, p, src) = self.cfg()?;
        let fs = suwayomi::fetch_filters(&base, &u, &p, &src)?;
        Ok(fs.into_iter().enumerate().filter_map(|(pos, f)| {
            let id = Cow::Owned(format!("{pos}"));
            match f {
                suwayomi::SwFilter::CheckBoxFilter { name, cbDefault: default } => Some(Filter {
                    id, title: Some(Cow::Owned(name)), hide_from_header: None,
                    kind: FilterKind::Check { name: None, can_exclude: false, default: Some(default) },
                }),
                suwayomi::SwFilter::SelectFilter { name, selDefault: default, values } => {
                    if values.is_empty() { return None; }
                    let def = values.get(default.max(0) as usize).cloned();
                    Some(Filter {
                        id, title: Some(Cow::Owned(name)), hide_from_header: None,
                        kind: FilterKind::Select {
                            is_genre: false, uses_tag_style: false,
                            options: values.into_iter().map(Cow::Owned).collect(),
                            ids: None, default: def.map(Cow::Owned),
                        },
                    })
                }
                suwayomi::SwFilter::TextFilter { name } => Some(Filter {
                    id, title: Some(Cow::Owned(name)), hide_from_header: None,
                    kind: FilterKind::Text { placeholder: None },
                }),
                suwayomi::SwFilter::Other => None,
            }
        }).collect())
    }
}

impl Source for Bridge {
    fn new() -> Self { Self }
    fn get_search_manga_list(&self, query: Option<String>, page: i32, f: Vec<FilterValue>) -> Result<MangaPageResult> {
        let (base, u, p, src) = self.cfg()?;
        let q = query.unwrap_or_default();
        if q.trim_start().starts_with("http") {
            let r = suwayomi::add_by_url(&base, &u, &p, q.trim())?;
            if let Some(m) = r.manga {
                return Ok(MangaPageResult { entries: Vec::from([Self::manga(&m, &base)]), has_next_page: false });
            }
            return Err(error!("add-by-url: {:?}", r.message.unwrap_or_default()));
        }
        let changes: Vec<String> = if f.is_empty() {
            Vec::new()
        } else {
            let defs = suwayomi::fetch_filters(&base, &u, &p, &src).unwrap_or_default();
            f.iter().filter_map(|fv| {
                let pos: usize = match fv {
                    FilterValue::Check { id, .. }
                    | FilterValue::Text { id, .. }
                    | FilterValue::Select { id, .. } => id.parse().ok()?,
                    _ => return None,
                };
                to_change(pos, fv, &defs)
            }).collect()
        };
        let d = suwayomi::fetch_source(&base, &u, &p, &src, "SEARCH", Some(&q), page, &changes)?;
        Ok(MangaPageResult { entries: d.mangas.iter().map(|m| Self::manga(m, &base)).collect(),
            has_next_page: d.has_next_page.unwrap_or(false) })
    }
    fn get_manga_update(&self, mut manga: Manga, need_d: bool, need_c: bool) -> Result<Manga> {
        let (base, u, p, _s) = self.cfg()?;
        let id: i32 = manga.key.parse().map_err(|_| error!("bad manga key"))?;
        if need_c {
            let cs = suwayomi::fetch_chapters(&base, &u, &p, id)?;
            manga.chapters = Some(cs.into_iter().map(|c| Chapter {
                key: format!("{}", c.id), title: Some(c.name.clone()),
                chapter_number: c.chapter_number,
                date_uploaded: c.upload_date.as_deref().and_then(|s| s.parse::<i64>().ok()).map(|ms| ms/1000),
                scanlators: c.scanlator.filter(|s| !s.is_empty()).map(|s| Vec::from([s])),
                url: c.real_url, ..Default::default() }).collect());
        }
        if need_d {
            if let Ok(d) = suwayomi::fetch_manga_details(&base, &u, &p, id) {
                if !d.title.is_empty() { manga.title = d.title.clone(); }
                if d.description.as_deref().map(|s| !s.is_empty()).unwrap_or(false) { manga.description = d.description.clone(); }
                if let Some(a) = d.author.filter(|s| !s.is_empty()) { manga.authors = Some(Vec::from([a])); }
                if let Some(a) = d.artist.filter(|s| !s.is_empty()) { manga.artists = Some(Vec::from([a])); }
                if let Some(g) = d.genre.filter(|v| !v.is_empty()) { manga.tags = Some(g); }
                match d.status.as_deref() {
                    Some("ONGOING") => manga.status = MangaStatus::Ongoing,
                    Some("COMPLETED") => manga.status = MangaStatus::Completed,
                    Some("HIATUS") => manga.status = MangaStatus::Hiatus,
                    Some("CANCELLED") => manga.status = MangaStatus::Cancelled,
                    _ => {}
                }
            }
        }
        Ok(manga)
    }
    fn get_page_list(&self, _m: Manga, ch: Chapter) -> Result<Vec<Page>> {
        let (base, u, p, _s) = self.cfg()?;
        let id: i32 = ch.key.parse().map_err(|_| error!("bad chapter key"))?;
        Ok(suwayomi::fetch_pages(&base, &u, &p, id)?.into_iter()
            .map(|x| Page { content: PageContent::url(suwayomi::abs(&base, &x)), ..Default::default() }).collect())
    }
}

impl ListingProvider for Bridge {
    fn get_manga_list(&self, listing: Listing, page: i32) -> Result<MangaPageResult> {
        let (base, u, p, src) = self.cfg()?;
        let ty = match listing.id.as_str() { "popular" => "POPULAR", "latest" => "LATEST",
            _ => return Err(error!("unknown listing {}", listing.id)) };
        let d = suwayomi::fetch_source(&base, &u, &p, &src, ty, None, page, &[])?;
        Ok(MangaPageResult { entries: d.mangas.iter().map(|m| Self::manga(m, &base)).collect(),
            has_next_page: d.has_next_page.unwrap_or(false) })
    }
}

impl ImageRequestProvider for Bridge {
    fn get_image_request(&self, url: String, _ctx: Option<aidoku::HashMap<String, String>>) -> Result<Request> {
        let (base, u, p, _s) = self.cfg().unwrap_or((String::from(""), None, None, String::from("")));
        let mut req = Request::get(&url).map_err(|e| error!("{:?}", e))?;
        if !base.is_empty() && url.starts_with(base.as_str()) {
            if let Some(a) = suwayomi::auth_val(&u, &p) { req.set_header("Authorization", &a); }
            req.set_header("Referer", &base);
        }
        Ok(req)
    }
}

register_source!(Bridge, ListingProvider, ImageRequestProvider, DynamicFilters);
