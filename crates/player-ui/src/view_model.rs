#[derive(Clone, Debug)]
pub struct MediaPreview {
    pub id: i32,
    pub title: String,
    pub artist: String,
    pub collection: String,
    pub duration: String,
    pub cover: i32,
}

#[derive(Clone, Debug)]
pub struct ShellViewModel {
    pub page: i32,
    pub selected_id: i32,
    pub selected_title: String,
    pub selected_artist: String,
    pub selected_duration: String,
    pub selected_cover: i32,
    pub volume_percent: f32,
    pub favorite: bool,
    pub status: String,
    pub tracks: Vec<MediaPreview>,
    pub queue: Vec<MediaPreview>,
}
