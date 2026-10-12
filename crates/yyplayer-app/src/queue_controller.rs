use super::*;
use player_core::queue::{PlayOrder, QueueKind, SessionQueue};

impl AppController {
    pub(super) fn queue_for(&self, kind: QueueKind) -> &SessionQueue {
        if kind == self.queue_kind {
            &self.queue
        } else {
            &self.other_queue
        }
    }
    fn queue_for_mut(&mut self, kind: QueueKind) -> &mut SessionQueue {
        if kind == self.queue_kind {
            &mut self.queue
        } else {
            &mut self.other_queue
        }
    }
    pub(super) fn switch_queue(&mut self, kind: QueueKind) {
        if self.queue_kind != kind {
            std::mem::swap(&mut self.queue, &mut self.other_queue);
            self.queue_kind = kind;
            self.selected_id = self.queue.selected.map(|i| i as i32).unwrap_or(-1);
            self.queue_version += 1;
        }
    }
    pub(super) fn replace_queue(&mut self, kind: QueueKind, sources: Vec<MediaSource>) -> bool {
        if let Err(e) = self.queue_for_mut(kind).replace(sources) {
            self.status = e;
            return false;
        }
        self.switch_queue(kind);
        self.selected_id = -1;
        self.queue_version += 1;
        true
    }
    pub(super) fn enqueue(&mut self, kind: QueueKind, sources: Vec<MediaSource>) {
        if kind == QueueKind::Music && self.online.streaming && self.queue_kind == kind {
            self.status = "当前为联网串流，请先播放本地歌曲后再添加到队列".into();
            return;
        }
        match self.queue_for_mut(kind).append(sources) {
            Ok(n) => {
                self.queue_version += 1;
                self.queue_tab = kind;
                self.queue_open = true;
                self.status = format!(
                    "已添加 {n} 项到{}播放列表",
                    if kind == QueueKind::Music {
                        "音乐"
                    } else {
                        "视频"
                    }
                );
            }
            Err(e) => self.status = e,
        }
    }
    fn order_for(&self, kind: QueueKind) -> PlayOrder {
        if kind == QueueKind::Music {
            self.settings.music_order
        } else {
            self.settings.video_order
        }
    }
    pub(super) fn advance_queue(&mut self, forward: bool, automatic: bool) {
        let order = self.order_for(self.queue_kind);
        let before = self.selected_id;
        if let Some(i) = self.queue.adjacent(forward, automatic, order) {
            if automatic && i as i32 == before {
                self.send(PlaybackCommand::Seek(Duration::ZERO));
                self.send(PlaybackCommand::Resume);
            } else {
                self.load_index(i as i32);
            }
        }
    }
    pub(super) fn queue_control(&mut self, action: &str, value: &str) -> bool {
        let index = value.parse::<usize>().ok();
        match action {
            "queue-toggle" => {
                self.queue_open = !self.queue_open;
                if self.queue_open {
                    self.panel_open = false;
                    self.queue_tab = if matches!(self.navigation.current(), 1 | 5) {
                        QueueKind::Video
                    } else {
                        QueueKind::Music
                    };
                }
            }
            "queue-close" => self.queue_open = false,
            "queue-tab" => {
                self.queue_tab = if value == "1" {
                    QueueKind::Video
                } else {
                    QueueKind::Music
                }
            }
            "queue-play" => {
                if let Some(i) = index
                    && i < self.queue_for(self.queue_tab).len()
                {
                    self.switch_queue(self.queue_tab);
                    self.preview = false;
                    self.load_index(i as i32);
                }
            }
            "queue-order" | "loop" => {
                let kind = if action == "loop" {
                    self.queue_kind
                } else {
                    self.queue_tab
                };
                let order = if action == "loop" {
                    if value == "yes" {
                        PlayOrder::RepeatOne
                    } else {
                        PlayOrder::Sequential
                    }
                } else {
                    PlayOrder::from_index(value.parse().unwrap_or(0))
                };
                if kind == QueueKind::Music {
                    self.settings.music_order = order;
                } else {
                    self.settings.video_order = order;
                }
                self.queue_for_mut(kind).reset_traversal();
                self.send(PlaybackCommand::SetLoop(false));
                self.settings_dirty = Some(Instant::now());
            }
            "queue-remove" => {
                if let Some(i) = index {
                    self.queue_for_mut(self.queue_tab).remove(i);
                    self.selected_id = self.queue.selected.map(|i| i as i32).unwrap_or(-1);
                    self.queue_version += 1;
                }
            }
            "queue-up" | "queue-down" => {
                if let Some(i) = index {
                    self.queue_for_mut(self.queue_tab)
                        .move_item(i, if action == "queue-up" { -1 } else { 1 });
                    self.selected_id = self.queue.selected.map(|i| i as i32).unwrap_or(-1);
                    self.queue_version += 1;
                }
            }
            "queue-clear" => {
                if self.queue_tab == self.queue_kind {
                    self.control("stop", "");
                }
                self.queue_for_mut(self.queue_tab).clear();
                self.selected_id = self.queue.selected.map(|i| i as i32).unwrap_or(-1);
                self.queue_version += 1;
            }
            _ => return false,
        }
        true
    }
    pub(super) fn drawer_view(&self) -> player_ui::view_model::QueueViewModel {
        let cache = self.queue_projection.borrow();
        let active = self.queue_tab == self.queue_kind;
        player_ui::view_model::QueueViewModel {
            open: self.queue_open,
            tab: self.queue_tab.index(),
            order: self.order_for(self.queue_tab).index(),
            rows: if active {
                cache.queue.clone()
            } else {
                cache.other.clone()
            },
            revision: cache.revision,
            selected: if active { self.selected_id } else { -1 },
            music_count: self.queue_for(QueueKind::Music).len() as i32,
            video_count: self.queue_for(QueueKind::Video).len() as i32,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn controller() -> AppController {
        let mut c = AppController::new(MpvEngine::default(), player_platform::current());
        c.preview = false;
        c
    }
    #[test]
    fn independent_sessions_edits_modes_and_escape() {
        let mut c = controller();
        assert!(c.replace_queue(
            QueueKind::Music,
            vec![
                MediaSource::Local("a.wav".into()),
                MediaSource::Local("b.wav".into())
            ]
        ));
        c.queue.select(1);
        c.selected_id = 1;
        assert!(c.replace_queue(QueueKind::Video, vec![MediaSource::Local("v.mp4".into())]));
        c.queue.select(0);
        c.selected_id = 0;
        c.enqueue(QueueKind::Music, vec![MediaSource::Local("c.wav".into())]);
        assert_eq!(c.queue.len(), 1);
        assert_eq!(c.other_queue.len(), 3);
        c.control("queue-order", "3");
        assert_eq!(c.settings.music_order, PlayOrder::Shuffle);
        assert_eq!(c.settings.video_order, PlayOrder::Sequential);
        c.switch_queue(QueueKind::Music);
        assert_eq!(c.selected_id, 1);
        c.control("queue-up", "1");
        assert_eq!(c.selected_id, 0);
        assert_eq!(c.queue[0], MediaSource::Local("b.wav".into()));
        c.control("queue-remove", "2");
        assert_eq!(c.queue.len(), 2);
        assert_eq!(c.other_queue.len(), 1);
        assert!(c.key("Escape", true, false, true));
        assert!(!c.queue_open);
    }
}
