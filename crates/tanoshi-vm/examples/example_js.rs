use tanoshi_lib::{models::Input, prelude::TriState};
use tanoshi_vm::extension::manager::SourceManager;

#[tokio::main]
async fn main() {
    env_logger::init();

    let source_name = "MangaLife";

    let manager = SourceManager::new("C:\\Users\\fadhlika\\Repos\\tanoshi-extensions\\dist");
    manager.load(source_name).await.unwrap();

    let extension = manager.get(4).unwrap();

    let source_info = extension.get_source_info();
    println!("{:?}", source_info);

    let prefs = extension.get_preferences().await.unwrap();
    println!("{:?}", prefs);

    let mut filters = extension.get_filter_list().await.unwrap();
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

    let manga = extension
        .search_manga(1, None, Some(filters))
        .await
        .unwrap();

    println!("{:?}", manga);

    let manga = extension.get_latest_manga(1).await.unwrap();

    println!("{:?}", manga);

    let manga = extension.get_popular_manga(1).await.unwrap();

    println!("{:?}", manga);

    let manga = extension
        .get_manga_detail(manga[2].path.clone())
        .await
        .unwrap();

    println!("{:?}", manga);

    let chapters = extension.get_chapters(manga.path.clone()).await.unwrap();

    println!("{:?}", chapters);

    let pages = extension.get_pages(chapters[0].path.clone()).await.unwrap();

    println!("{:?}", pages);
}
