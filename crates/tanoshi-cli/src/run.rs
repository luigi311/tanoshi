use tanoshi_lib::prelude::Input;
use tanoshi_vm::prelude::SourceManager;

pub async fn run(manager: SourceManager, name: &str) -> Result<(), Box<dyn std::error::Error>> {
    manager.load(name)?;

    let extension = manager.get(4)?;
    let prefs = extension.get_preferences().unwrap();
    println!("{:?}", prefs);

    let mut filters = extension.get_filter_list().unwrap();
    println!("{:?}", filters);

    for filter in filters.iter_mut() {
        match filter {
            Input::Text { state, .. } => *state = Some("One Piece".to_string()),
            Input::Group { state, .. } => *state = Some(vec!["Romance".into()]),
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

    Ok(())
}
