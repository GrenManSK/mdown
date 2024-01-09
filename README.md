# mdown

> mangadex manga downloader

See [site](https://mangadex.org/) for finding manga

---

## usage

`--url [String]` - url of manga

`--lang [String]` - language of manga to download

`--offset [Integer]` - changes start offset e.g. 50 starts from chapter 50

`--database-offset [Integer]` - changes start offset e.g. 50 starts from item 50 in database

`--force` - will download manga even if it already exists

`--title [String]` - will name the manga

`--folder [String]` - will put manga in folder specified

- if folder name is **name** it will put in folder same as manga name
- if folder name is **name** and title is specified it will make folder same as title

`--volume [Integer]` - will download manga which has supplied volume in it

`--chapter [Integer]` - will download manga which has supplied chapter in it

`--max-consecutive [Integer]` - will download manga images by supplied number at once; it is highly recommended to use **MAX *50*** (default is *40*) because of lack of performance and non complete manga downloading, meaning chapter will not download correctly, meaning missing pages

`--saver` - will download images of lower quality and lower download size; will save network resources and reduce download time

`--force-delete` - will force to delete *.lock file which is stopping from running another instance of program; NOTE that if you already have one instance running it will fail to delete the original file and thus it will crash

`--cwd` - change current working directory

`--stat` - will add txt file which contains status information

`--web` - will enter web mode and will open browser on port 8080, core lock file will not be initialized; result will be printed at end of download process

`--encode` - will print url in program readable format

`--log` - will print progress requests when received, web flag need to be set for this to work

---

- If you get message that about lock file is present and you believe you don't have already program started use `--force-delete` option to force it to delete lock file

- Will download cover image and description even if it did NOT download any more chapters in currently downloaded files AND if it do NOT find any eligible manga chapters it will delete the original
  - e.g. whole manga was in Japanese and didn't find any English chapters which results in 0 downloads

- Every non-final downloads and temporary files will be put in .cache folder which if empty will be deleted afterwards
