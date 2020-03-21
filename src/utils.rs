use crate::model::{Chapter, Document, History, Manga, Track};

pub fn merge_library(
    _key: &[u8],              // the key being merged
    old_value: Option<&[u8]>, // the previous value, if one existed
    merged_bytes: &[u8],      // the new bytes being merged in
) -> Option<Vec<u8>> {
    let ret = old_value.map(|ov| ov.to_vec()).unwrap_or(vec![]);
    let mut old = serde_json::from_slice(&ret).unwrap_or(Document::default());

    if let Ok(chapter) = serde_json::from_slice::<Chapter>(&merged_bytes) {
        match old.chapters.iter().position(|ch| ch == &chapter) {
            Some(idx) => old.chapters[idx] = chapter,
            None => old.chapters.push(chapter),
        };
    } else if let Ok(track) = serde_json::from_slice::<Track>(&merged_bytes) {
        match old.tracks.iter().position(|tr| tr == &track) {
            Some(idx) => old.tracks[idx] = track,
            None => old.tracks.push(track),
        };
    } else if let Ok(history) = serde_json::from_slice::<History>(&merged_bytes) {
        match old.history.iter().position(|his| his == &history) {
            Some(idx) => old.history[idx] = history,
            None => old.history.push(history),
        };
    }

    Some(serde_json::to_vec(&old).unwrap())
}
