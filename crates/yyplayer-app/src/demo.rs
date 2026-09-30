//! Presentation-only examples. These are not files, library records or engine metadata.

use player_ui::view_model::MediaPreview;

pub fn tracks() -> Vec<MediaPreview> {
    [
        ("Midnight Bloom", "YYPlayer Sessions", "夜色之间", "4:38", 0),
        ("Still Waters", "Studio North", "慢下来", "3:52", 1),
        ("Golden Hour", "The Sunday Project", "日光片刻", "5:16", 2),
        (
            "A Little Further",
            "YYPlayer Sessions",
            "夜色之间",
            "4:07",
            0,
        ),
    ]
    .into_iter()
    .enumerate()
    .map(
        |(index, (title, artist, collection, duration, cover))| MediaPreview {
            id: index as i32,
            title: title.into(),
            artist: artist.into(),
            collection: collection.into(),
            duration: duration.into(),
            cover,
        },
    )
    .collect()
}
