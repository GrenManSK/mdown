# mdown

mangadex manga downloader

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

---

- If you get message that about lock file is present adn you believe you don't have already program started use `--force-delete` option to force it to delete lock file

---

- Download size on right side is not correctly calculated; beware of that that current download size may be higher than final download size
