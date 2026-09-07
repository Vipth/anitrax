//! Raw GraphQL documents. Kept together so the request shapes are easy to audit
//! against AniList's rate limit — every one of these is a *single* POST.

pub const MEDIA_FIELDS: &str = r#"
  id
  title { romaji english native }
  format
  status
  description(asHtml: false)
  episodes
  duration
  season
  seasonYear
  averageScore
  genres
  synonyms
  coverImage { large color }
  bannerImage
  siteUrl
  startDate { year month day }
  nextAiringEpisode { episode airingAt }
"#;

/// The current authenticated user.
pub const VIEWER: &str = r#"
query {
  Viewer {
    id
    name
    avatar { large }
    mediaListOptions { scoreFormat }
  }
}
"#;

/// The whole anime list in one request. AniList embeds full media objects here,
/// so this single call warms the entire media cache too.
pub fn media_list_collection() -> String {
    format!(
        r#"
query ($userId: Int!) {{
  MediaListCollection(userId: $userId, type: ANIME) {{
    lists {{
      name
      isCustomList
      entries {{
        id
        status
        progress
        score(format: POINT_100)
        repeat
        notes
        updatedAt
        startedAt {{ year month day }}
        completedAt {{ year month day }}
        media {{ {fields} }}
      }}
    }}
  }}
}}
"#,
        fields = MEDIA_FIELDS
    )
}

pub fn save_media_list_entry() -> String {
    // Dates are intentionally omitted for the first milestone.
    format!(
        r#"
mutation (
  $mediaId: Int!
  $status: MediaListStatus
  $progress: Int
  $scoreRaw: Int
  $repeat: Int
  $notes: String
) {{
  SaveMediaListEntry(
    mediaId: $mediaId
    status: $status
    progress: $progress
    scoreRaw: $scoreRaw
    repeat: $repeat
    notes: $notes
  ) {{
    id
    status
    progress
    score(format: POINT_100)
    repeat
    notes
    updatedAt
    media {{ {fields} }}
  }}
}}
"#,
        fields = MEDIA_FIELDS
    )
}

pub const DELETE_MEDIA_LIST_ENTRY: &str = r#"
mutation ($id: Int!) {
  DeleteMediaListEntry(id: $id) { deleted }
}
"#;

pub fn search_media() -> String {
    format!(
        r#"
query ($q: String!, $perPage: Int!) {{
  Page(page: 1, perPage: $perPage) {{
    media(search: $q, type: ANIME, sort: SEARCH_MATCH, isAdult: false) {{
      {fields}
    }}
  }}
}}
"#,
        fields = MEDIA_FIELDS
    )
}

/// Build one aliased query that fetches many media objects in a single request.
pub fn batch_media(ids: &[i64]) -> String {
    let selections: String = ids
        .iter()
        .enumerate()
        .map(|(i, id)| format!("  m{i}: Media(id: {id}, type: ANIME) {{ {MEDIA_FIELDS} }}\n"))
        .collect();
    format!("query {{\n{selections}}}")
}
