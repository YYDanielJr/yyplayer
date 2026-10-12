//! Bounded session queue traversal; the controller alone executes EOF advancement.
use crate::MediaSource;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum PlayOrder {
    #[default]
    Sequential,
    RepeatAll,
    RepeatOne,
    Shuffle,
}
impl PlayOrder {
    pub fn index(self) -> i32 {
        match self {
            Self::Sequential => 0,
            Self::RepeatAll => 1,
            Self::RepeatOne => 2,
            Self::Shuffle => 3,
        }
    }
    pub fn from_index(i: i32) -> Self {
        match i {
            1 => Self::RepeatAll,
            2 => Self::RepeatOne,
            3 => Self::Shuffle,
            _ => Self::Sequential,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum QueueKind {
    #[default]
    Music,
    Video,
}
impl QueueKind {
    pub fn index(self) -> i32 {
        if self == Self::Video { 1 } else { 0 }
    }
}

#[derive(Clone, Debug, Default)]
pub struct SessionQueue {
    items: Vec<MediaSource>,
    pub selected: Option<usize>,
    bag: Vec<usize>,
    history: Vec<usize>,
    history_pos: usize,
    rng: u64,
}
impl std::ops::Deref for SessionQueue {
    type Target = [MediaSource];
    fn deref(&self) -> &Self::Target {
        &self.items
    }
}
impl SessionQueue {
    pub const LIMIT: usize = 20_000;
    pub fn replace(&mut self, items: Vec<MediaSource>) -> Result<(), String> {
        if items.len() > Self::LIMIT {
            return Err("播放列表最多 20000 项".into());
        }
        self.items = items;
        self.selected = None;
        self.reset_traversal();
        Ok(())
    }
    pub fn append(&mut self, items: Vec<MediaSource>) -> Result<usize, String> {
        // Preserve existing duplicates (each is a deliberate queue position), but
        // batch additions are idempotent by original media identity.
        let mut known: std::collections::HashSet<_> = self.items.iter().cloned().collect();
        let added: Vec<_> = items
            .into_iter()
            .filter(|s| known.insert(s.clone()))
            .collect();
        if self.items.len() + added.len() > Self::LIMIT {
            return Err("播放列表最多 20000 项".into());
        }
        let n = added.len();
        self.items.extend(added);
        self.reset_traversal();
        Ok(n)
    }
    pub fn clear(&mut self) {
        self.items.clear();
        self.selected = None;
        self.reset_traversal();
    }
    pub fn select(&mut self, index: usize) {
        if index < self.len() {
            self.selected = Some(index);
            self.reset_traversal();
        }
    }
    pub fn reset_traversal(&mut self) {
        self.bag.clear();
        self.history.clear();
        self.history_pos = 0;
        if let Some(i) = self.selected {
            self.history.push(i);
        }
    }
    pub fn seed(&mut self, seed: u64) {
        self.rng = seed.max(1);
    }
    pub fn remove(&mut self, index: usize) {
        if index >= self.len() {
            return;
        }
        self.items.remove(index);
        self.selected = self.selected.and_then(|s| {
            if s == index {
                None
            } else {
                Some(s - usize::from(s > index))
            }
        });
        self.reset_traversal();
    }
    pub fn move_item(&mut self, index: usize, offset: i32) {
        let next = index as i64 + i64::from(offset);
        if index >= self.len() || next < 0 || next as usize >= self.len() {
            return;
        }
        let next = next as usize;
        self.items.swap(index, next);
        self.selected = self.selected.map(|s| {
            if s == index {
                next
            } else if s == next {
                index
            } else {
                s
            }
        });
        self.reset_traversal();
    }
    pub fn adjacent(&mut self, forward: bool, automatic: bool, order: PlayOrder) -> Option<usize> {
        if self.is_empty() {
            return None;
        }
        if automatic && order == PlayOrder::RepeatOne {
            return self.selected.or(Some(0));
        }
        if order == PlayOrder::Shuffle {
            if !forward {
                if self.history_pos == 0 {
                    return None;
                }
                self.history_pos -= 1;
                let i = self.history[self.history_pos];
                self.selected = Some(i);
                return Some(i);
            }
            if self.history_pos + 1 < self.history.len() {
                self.history_pos += 1;
                let i = self.history[self.history_pos];
                self.selected = Some(i);
                return Some(i);
            }
            if self.bag.is_empty() {
                self.bag = (0..self.len())
                    .filter(|i| Some(*i) != self.selected)
                    .collect();
                for end in (1..self.bag.len()).rev() {
                    // Small session PRNG, not used for security or file identity.
                    self.rng = self.rng.max(1);
                    self.rng ^= self.rng << 13;
                    self.rng ^= self.rng >> 7;
                    self.rng ^= self.rng << 17;
                    self.bag.swap(end, (self.rng % (end as u64 + 1)) as usize);
                }
            }
            let i = self.bag.pop().or(self.selected).unwrap_or(0);
            if self.history.len() >= Self::LIMIT {
                self.history.remove(0);
            }
            self.history.push(i);
            self.history_pos = self.history.len() - 1;
            self.selected = Some(i);
            return Some(i);
        }
        let next = match self.selected {
            Some(i) => i as i64 + if forward { 1 } else { -1 },
            None => 0,
        };
        let wrap = matches!(order, PlayOrder::RepeatAll | PlayOrder::RepeatOne);
        if !wrap && (next < 0 || next >= self.len() as i64) {
            return None;
        }
        let i = next.rem_euclid(self.len() as i64) as usize;
        self.selected = Some(i);
        Some(i)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn queue(n: usize) -> SessionQueue {
        let mut q = SessionQueue::default();
        q.seed(12345);
        q.replace(
            (0..n)
                .map(|i| MediaSource::Local(format!("{i}.wav").into()))
                .collect(),
        )
        .unwrap();
        q.select(0);
        q
    }
    #[test]
    fn eof_and_manual_modes() {
        let mut q = queue(2);
        assert_eq!(q.adjacent(true, true, PlayOrder::RepeatOne), Some(0));
        assert_eq!(q.adjacent(true, false, PlayOrder::RepeatOne), Some(1));
        assert_eq!(q.adjacent(true, true, PlayOrder::Sequential), None);
        assert_eq!(q.adjacent(true, true, PlayOrder::RepeatAll), Some(0));
        assert_eq!(q.adjacent(false, false, PlayOrder::RepeatAll), Some(1));
    }
    #[test]
    fn shuffle_visits_each_other_song_once_and_previous_retraces() {
        let mut q = queue(20);
        let mut seen = std::collections::BTreeSet::new();
        let first = q.adjacent(true, true, PlayOrder::Shuffle).unwrap();
        seen.insert(first);
        for _ in 0..18 {
            assert!(seen.insert(q.adjacent(true, true, PlayOrder::Shuffle).unwrap()));
        }
        assert_eq!(seen.len(), 19);
        assert!(!seen.contains(&0));
        let last = q.selected.unwrap();
        let prior = q.adjacent(false, false, PlayOrder::Shuffle).unwrap();
        assert_ne!(prior, last);
        assert_eq!(q.adjacent(true, false, PlayOrder::Shuffle), Some(last));
        assert_ne!(q.adjacent(true, true, PlayOrder::Shuffle), Some(last));
        assert_eq!(queue(1).adjacent(true, true, PlayOrder::Shuffle), Some(0));
        assert_eq!(queue(0).adjacent(true, true, PlayOrder::Shuffle), None);
    }
    #[test]
    fn edits_preserve_current_source_and_reject_overflow_atomically() {
        let mut q = queue(3);
        q.select(1);
        let current = q[1].clone();
        q.move_item(1, 1);
        assert_eq!(q[q.selected.unwrap()], current);
        q.remove(0);
        assert_eq!(q[q.selected.unwrap()], current);
        q.remove(q.selected.unwrap());
        assert_eq!(q.selected, None);
        let before = q.len();
        assert!(
            q.append(
                (0..SessionQueue::LIMIT + 1)
                    .map(|i| MediaSource::Url(i.to_string()))
                    .collect()
            )
            .is_err()
        );
        assert_eq!(q.len(), before);
    }
}
