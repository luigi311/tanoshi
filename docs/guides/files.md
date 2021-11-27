# Local Manga Files

Manga files have to be structured below, it tested for `cbz` and `cbr` files. Tanoshi support single archive file, archive files inside a series folder, and image folders inside a series folder.

!!! warning
    
    While single archive will be considered a series, a folder with images inside will not considered a series.


!!! info "Series"

    Every folder or archive inside manga directory will be considered a series.

!!! info "Chapters"

    Every folder or archive inside a series folder will be considered a chapter

!!! info "Page"

    Every file inside archive or a chapter folder will be considered a page.


```
/path/to/manga
├─── Series 1
│    ├─── Volume 1.cbz
|    ├─── Volume 2.cbz
|    └─── ...
├─── Series 2
|    ├─── Volume 1.cbr
|    ├─── Volume 2.cbr
|    └─── ...
├─── Series 3
|    ├─── Volume 1.cbr
|    ├─── Volume 2.cbz
|    ├─── Volume 3
|    |    ├─── Page 1.png
|    |    ├─── Page 2.png
|    |    └─── Page 3.png
|    └─── ...
├─── Series 4.cbz
└─── Series 5.cbr
```
