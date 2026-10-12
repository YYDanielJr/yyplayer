//! Persistent local playlists. Paths, not labels, are song identities.
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, path::PathBuf};

pub const FAVORITES: u64 = 1;
pub const MAX_TRACKS: usize = 20_000;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Playlist {
    pub id: u64,
    pub name: String,
    #[serde(with = "crate::path_serde::vec")]
    pub tracks: Vec<PathBuf>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct Playlists {
    pub version: u32,
    pub next_id: u64,
    pub lists: Vec<Playlist>,
}
impl Default for Playlists {
    fn default() -> Self {
        Self {
            version: 1,
            next_id: 2,
            lists: vec![Playlist {
                id: FAVORITES,
                name: "我喜欢".into(),
                tracks: vec![],
            }],
        }
    }
}
fn validate_name(name: &str) -> Result<(), String> {
    if name.trim().is_empty() || name.chars().count() > 80 || name.chars().any(char::is_control) {
        Err("歌单名称须为 1–80 个字符，不能包含控制字符".into())
    } else {
        Ok(())
    }
}
impl Playlists {
    pub fn validate(&self) -> Result<(), String> {
        if !self
            .lists
            .first()
            .is_some_and(|p| p.id == FAVORITES && p.name == "我喜欢")
        {
            return Err("我喜欢歌单身份无效".into());
        }
        if self.version != 1 || self.lists.is_empty() || self.lists.len() > 128 {
            return Err("歌单版本无效或数量超过 128 个".into());
        }
        let mut ids = BTreeSet::new();
        let mut names = BTreeSet::new();
        let mut total = 0;
        for list in &self.lists {
            validate_name(&list.name)?;
            if list.id == 0
                || !ids.insert(list.id)
                || !names.insert(list.name.trim().to_lowercase())
            {
                return Err("歌单名称或身份重复".into());
            }
            let paths: BTreeSet<_> = list.tracks.iter().collect();
            if paths.len() != list.tracks.len() || paths.iter().any(|p| p.as_os_str().is_empty()) {
                return Err("歌单含空路径或重复歌曲".into());
            }
            total += list.tracks.len();
        }
        if total > MAX_TRACKS || !ids.contains(&FAVORITES) || self.next_id <= *ids.last().unwrap() {
            return Err("歌单容量或身份序列无效（所有歌单合计最多 20000 首）".into());
        }
        Ok(())
    }
    pub fn create(&mut self, name: &str) -> Result<u64, String> {
        validate_name(name)?;
        if self.lists.len() >= 128
            || self
                .lists
                .iter()
                .any(|p| p.name.to_lowercase() == name.trim().to_lowercase())
        {
            return Err("歌单名称已存在或数量已达上限".into());
        }
        let next = self.next_id.checked_add(1).ok_or("歌单身份已达上限")?;
        let id = self.next_id;
        self.lists.push(Playlist {
            id,
            name: name.trim().into(),
            tracks: vec![],
        });
        self.next_id = next;
        Ok(id)
    }
    pub fn rename(&mut self, id: u64, name: &str) -> Result<(), String> {
        validate_name(name)?;
        if id == FAVORITES {
            return Err("我喜欢歌单保留固定名称".into());
        }
        if self
            .lists
            .iter()
            .any(|p| p.id != id && p.name.to_lowercase() == name.trim().to_lowercase())
        {
            return Err("歌单名称已存在".into());
        }
        self.lists
            .iter_mut()
            .find(|p| p.id == id)
            .ok_or("歌单已不存在")?
            .name = name.trim().into();
        Ok(())
    }
    pub fn delete(&mut self, id: u64) -> Result<(), String> {
        if id == FAVORITES {
            return Err("我喜欢歌单不能删除".into());
        }
        self.lists.retain(|p| p.id != id);
        Ok(())
    }
    pub fn add(&mut self, id: u64, paths: Vec<PathBuf>) -> Result<usize, String> {
        let total: usize = self.lists.iter().map(|p| p.tracks.len()).sum();
        let list = self
            .lists
            .iter_mut()
            .find(|p| p.id == id)
            .ok_or("歌单已不存在")?;
        let mut known: BTreeSet<_> = list.tracks.iter().cloned().collect();
        let added: Vec<_> = paths
            .into_iter()
            .filter(|p| !p.as_os_str().is_empty() && known.insert(p.clone()))
            .collect();
        if total + added.len() > MAX_TRACKS {
            return Err("所有歌单合计最多 20000 首".into());
        }
        let count = added.len();
        list.tracks.extend(added);
        Ok(count)
    }
    pub fn remove(&mut self, id: u64, paths: &BTreeSet<PathBuf>) {
        if let Some(list) = self.lists.iter_mut().find(|p| p.id == id) {
            list.tracks.retain(|p| !paths.contains(p));
        }
    }
    pub fn contains(&self, id: u64, path: &std::path::Path) -> bool {
        self.lists
            .iter()
            .find(|p| p.id == id)
            .is_some_and(|p| p.tracks.iter().any(|p| p == path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn identity_dedup_and_editing_are_stable() {
        let mut p = Playlists::default();
        let id = p.create("  夜航  ").unwrap();
        assert_eq!(
            p.add(id, vec!["a.flac".into(), "a.flac".into(), "b.flac".into()])
                .unwrap(),
            2
        );
        p.rename(id, "夜曲").unwrap();
        assert!(p.create("夜曲").is_err());
        assert!(p.delete(FAVORITES).is_err());
        p.remove(id, &BTreeSet::from(["a.flac".into()]));
        p.validate().unwrap();
        let restored: Playlists =
            serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
        assert_eq!(p, restored);
        p.delete(id).unwrap();
        assert!(p.create("夜曲").unwrap() > id);
    }
    #[test]
    fn bad_data_and_capacity_fail_without_partial_addition() {
        let mut p = Playlists::default();
        assert!(p.create(" \n ").is_err());
        assert!(p.create(&"曲".repeat(81)).is_err());
        p.lists[0].tracks = (0..MAX_TRACKS)
            .map(|i| PathBuf::from(format!("{i}.wav")))
            .collect();
        assert!(p.add(FAVORITES, vec!["extra.wav".into()]).is_err());
        assert_eq!(p.lists[0].tracks.len(), MAX_TRACKS);
        p.next_id = 1;
        assert!(p.validate().is_err());
    }
    #[cfg(unix)]
    #[test]
    fn non_utf8_song_paths_roundtrip() {
        use std::os::unix::ffi::OsStringExt;
        let path = PathBuf::from(std::ffi::OsString::from_vec(b"song-\xff.flac".to_vec()));
        let mut p = Playlists::default();
        p.add(FAVORITES, vec![path.clone()]).unwrap();
        let r: Playlists = serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
        assert!(r.contains(FAVORITES, &path));
    }
}
