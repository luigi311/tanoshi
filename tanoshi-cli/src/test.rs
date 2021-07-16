use tanoshi_lib::prelude::{Param, SortByParam, SortOrderParam};
use tanoshi_vm::extension_bus::ExtensionBus;

pub async fn test(bus: ExtensionBus) -> Result<(), Box<dyn std::error::Error>> {
    for detail in bus.list().await? {
        println!("Test {}", detail.name);

        let param = Param {
            keyword: None,
            genres: None,
            page: None,
            sort_by: Some(SortByParam::LastUpdated),
            sort_order: Some(SortOrderParam::Desc),
            auth: None,
        };

        print!("Test get_manga_list ");
        let manga = bus.get_manga_list(detail.id, param).await?;
        println!("ok");

        print!("Test get_manga_info {} ", manga[0].path.clone());
        let _ = bus.get_manga_info(detail.id, manga[0].path.clone()).await?;
        println!("ok");

        print!("Test get_chapters {} ", manga[0].path.clone());
        let chapters = bus.get_chapters(detail.id, manga[0].path.clone()).await?;
        println!("ok");

        print!("Test get_pages {} ", chapters[0].path.clone());
        let _ = bus.get_pages(detail.id, chapters[0].path.clone()).await?;
        println!("ok");
    }

    Ok(())
}
