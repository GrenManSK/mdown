# mdown

mangadex manga downloader

---

## usage

`--url [String]` - url of manga set

`--lang [String]` - language of manga to download

`--offset [Integer]` - changes start offset e.g. 50 starts from chapter 50

`--database-offset [Integer]` - changes start offset e.g. 50 starts from item 50 in database

`--force` - will download manga even if it already exists

`--volume` - will download manga which has supplied volume in it

`--chapter` - will download manga which has supplied chapter in it

`--pack` - will download manga images by supplied number at once; it is highly recommended to use **MAX *50*** (default is *40*) because of lack of performance and non complete manga downloading, meaning chapter will not download correctly, meaning missing pages
