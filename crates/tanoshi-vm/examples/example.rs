use tanoshi_lib::{models::Input, prelude::TriState};
use tanoshi_vm::extension::bus::SourceBus;

#[tokio::main]
async fn main() {
    env_logger::init();

    let source_name = "mangasee";
    let source_id = 3;

    let path = "C:\\Users\\fadhlika\\Repos\\tanoshi-extensions\\target\\debug";

    let bus = SourceBus::new(path);
    bus.load(source_name).await.unwrap();

    let source_info = bus.get_source_info(source_id).await.unwrap();
    println!("{:?}", source_info);

    let prefs = bus.get_preferences(source_id).await.unwrap();
    println!("{:?}", prefs);

    let mut filters = bus.filter_list(source_id).await.unwrap();
    println!("{:?}", filters);

    for filter in filters.iter_mut() {
        match filter {
            Input::Text { state, .. } => *state = Some("One Piece".to_string()),
            Input::Group { state, .. } => {
                *state = vec![Input::State {
                    name: "Romance".to_string(),
                    selected: Some(TriState::Included),
                }]
            }
            _ => todo!(),
        }
    }

    let manga = bus
        .search_manga(source_id, 1, None, Some(filters))
        .await
        .unwrap();

    println!("{:?}", manga);

    let manga = bus.get_latest_manga(source_id, 1).await.unwrap();

    println!("{:?}", manga);

    let manga = bus.get_popular_manga(source_id, 1).await.unwrap();

    println!("{:?}", manga);

    let manga = bus
        .get_manga_detail(source_id, manga[2].path.clone())
        .await
        .unwrap();

    println!("{:?}", manga);

    let chapters = bus
        .get_chapters(source_id, manga.path.clone())
        .await
        .unwrap();

    println!("{:?}", chapters);

    let pages = bus
        .get_pages(source_id, chapters[0].path.clone())
        .await
        .unwrap();

    println!("{:?}", pages);
}
