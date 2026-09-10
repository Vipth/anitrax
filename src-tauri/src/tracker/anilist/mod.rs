pub mod gateway;
mod map;
mod queries;

use serde_json::{json, Value};

use crate::error::{AppError, AppResult};
use crate::tracker::model::*;
use crate::tracker::TrackerService;

pub use gateway::{AniListGateway, BudgetSnapshot};

/// `"YYYY-MM-DD"` -> AniList `FuzzyDateInput`. An empty / unparseable string
/// becomes an all-null date, which clears the field on AniList.
fn fuzzy_date_input(s: &str) -> Value {
    let mut parts = s.split('-').filter_map(|p| p.parse::<i64>().ok());
    match (parts.next(), parts.next(), parts.next()) {
        (Some(y), Some(m), Some(d)) => json!({ "year": y, "month": m, "day": d }),
        _ => json!({ "year": null, "month": null, "day": null }),
    }
}

/// AniList implementation of [`TrackerService`]. Every method goes through the
/// shared [`AniListGateway`]; this type holds no `reqwest::Client` of its own.
#[derive(Clone)]
pub struct AniList {
    gw: AniListGateway,
}

impl AniList {
    pub fn new(gw: AniListGateway) -> Self {
        Self { gw }
    }

    pub fn gateway(&self) -> &AniListGateway {
        &self.gw
    }

    async fn call(&self, query: impl Into<String>, variables: Value, token: Option<&str>) -> AppResult<Value> {
        let body = json!({ "query": query.into(), "variables": variables });
        self.gw.query(body, token.map(str::to_owned)).await
    }
}

#[async_trait::async_trait]
impl TrackerService for AniList {
    fn kind(&self) -> ServiceKind {
        ServiceKind::AniList
    }

    async fn viewer(&self, token: &str) -> AppResult<Viewer> {
        let data = self.call(queries::VIEWER, json!({}), Some(token)).await?;
        let v = data.get("Viewer").ok_or_else(|| AppError::Api {
            service: "anilist".into(),
            message: "no Viewer in response".into(),
        })?;
        Ok(Viewer {
            service: ServiceKind::AniList,
            id: v.get("id").and_then(|x| x.as_i64()).unwrap_or(0),
            name: v
                .get("name")
                .and_then(|x| x.as_str())
                .unwrap_or("Unknown")
                .to_string(),
            avatar_url: v
                .get("avatar")
                .and_then(|a| a.get("large"))
                .and_then(|x| x.as_str())
                .map(str::to_owned),
            score_format: v
                .get("mediaListOptions")
                .and_then(|o| o.get("scoreFormat"))
                .and_then(|x| x.as_str())
                .unwrap_or("POINT_10")
                .to_string(),
        })
    }

    async fn full_list(&self, token: &str, user_id: i64) -> AppResult<Vec<MediaListEntry>> {
        let data = self
            .call(queries::media_list_collection(), json!({ "userId": user_id }), Some(token))
            .await?;

        let lists = data
            .get("MediaListCollection")
            .and_then(|c| c.get("lists"))
            .and_then(|l| l.as_array())
            .cloned()
            .unwrap_or_default();

        let mut out: Vec<MediaListEntry> = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for list in lists {
            // Skip custom lists — the same entry also appears in its status list.
            if list.get("isCustomList").and_then(|b| b.as_bool()).unwrap_or(false) {
                continue;
            }
            let entries = list.get("entries").and_then(|e| e.as_array()).cloned().unwrap_or_default();
            for e in entries {
                if let Some(entry) = map::entry(&e) {
                    if seen.insert(entry.media.id.id) {
                        out.push(entry);
                    }
                }
            }
        }
        Ok(out)
    }

    async fn save_entry(&self, token: &str, patch: &EntryPatch) -> AppResult<MediaListEntry> {
        let mut vars = json!({ "mediaId": patch.media_id });
        if let Some(s) = patch.status {
            vars["status"] = json!(s.as_str());
        }
        if let Some(p) = patch.progress {
            vars["progress"] = json!(p);
        }
        if let Some(s) = patch.score_raw {
            vars["scoreRaw"] = json!(s.clamp(0, 100));
        }
        if let Some(r) = patch.repeat {
            vars["repeat"] = json!(r);
        }
        if let Some(n) = &patch.notes {
            vars["notes"] = json!(n);
        }
        if let Some(d) = &patch.started_at {
            vars["startedAt"] = fuzzy_date_input(d);
        }
        if let Some(d) = &patch.completed_at {
            vars["completedAt"] = fuzzy_date_input(d);
        }

        let data = self.call(queries::save_media_list_entry(), vars, Some(token)).await?;
        let saved = data.get("SaveMediaListEntry").ok_or_else(|| AppError::Api {
            service: "anilist".into(),
            message: "no SaveMediaListEntry in response".into(),
        })?;
        map::entry(saved).ok_or_else(|| AppError::Api {
            service: "anilist".into(),
            message: "could not parse saved entry".into(),
        })
    }

    async fn delete_entry(&self, token: &str, remote_id: i64) -> AppResult<()> {
        self.call(queries::DELETE_MEDIA_LIST_ENTRY, json!({ "id": remote_id }), Some(token))
            .await?;
        Ok(())
    }

    async fn search(&self, token: Option<&str>, query: &str) -> AppResult<Vec<Media>> {
        let data = self
            .call(
                queries::search_media(),
                json!({ "q": query, "perPage": 20 }),
                token,
            )
            .await?;
        let media = data
            .get("Page")
            .and_then(|p| p.get("media"))
            .and_then(|m| m.as_array())
            .cloned()
            .unwrap_or_default();
        Ok(media.iter().filter_map(map::media).collect())
    }

    async fn media_batch(&self, token: Option<&str>, ids: &[i64]) -> AppResult<Vec<Media>> {
        if ids.is_empty() {
            return Ok(vec![]);
        }
        let mut out = Vec::with_capacity(ids.len());
        for chunk in ids.chunks(25) {
            let data = self.call(queries::batch_media(chunk), json!({}), token).await?;
            if let Some(obj) = data.as_object() {
                for (_alias, val) in obj {
                    if let Some(m) = map::media(val) {
                        out.push(m);
                    }
                }
            }
        }
        Ok(out)
    }

    async fn season(
        &self,
        token: Option<&str>,
        year: i32,
        season: MediaSeason,
        page: i32,
    ) -> AppResult<SeasonPage> {
        let data = self
            .call(
                queries::season_page(),
                json!({
                    "year": year,
                    "season": season.as_str(),
                    "page": page,
                    "perPage": 50,
                }),
                token,
            )
            .await?;
        let page_obj = data.get("Page");
        let has_next_page = page_obj
            .and_then(|p| p.get("pageInfo"))
            .and_then(|i| i.get("hasNextPage"))
            .and_then(|b| b.as_bool())
            .unwrap_or(false);
        let media = page_obj
            .and_then(|p| p.get("media"))
            .and_then(|m| m.as_array())
            .map(|a| a.iter().filter_map(map::media).collect())
            .unwrap_or_default();
        Ok(SeasonPage {
            media,
            has_next_page,
        })
    }
}
