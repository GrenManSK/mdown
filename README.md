# mdown

> mangadex manga downloader

See [site](https://mangadex.org/) for finding manga

## Install

Firstly you have to install [Rust](https://www.rust-lang.org/tools/install)

`cargo build -r` will compile app and put it in this location `target/release/mdown.exe`

`cargo run -r` will compile and run app

`cargo run -r --` after this you can put arguments that will be pushed to the app see [usage](https://github.com/GrenManSK/mdown?tab=readme-ov-file#usage)

If you have EXE file in CWD (current working directory) all you need to do is run `mdown` or with arguments e.g. `mdown --url [UUID]`

### Features

- web (default)
- server (default)
- gui
- music
- full (contains all features)

To add feature run `cargo build -r -F [feature]`

If you want to add more features run `cargo build -r -F [feature1] -F [feature2]`

**IMPORTANT**  If you want to use music feature, you need to download music zip file from [pre-release](https://github.com/GrenManSK/mdown/releases/tag/music) and extract it to `resources/music/`

---

## usage

`--url [String]` - url of manga

`--lang [String]` - language of manga to download; "*" is for all languages

`--title [String]` - name the manga

`--folder [String]` - will put manga in folder specified

- if folder name is "**name**" it will put in folder same as manga name
- if folder name is "**name**" and title is specified it will make folder same as title

`--volume [Integer]` - will download manga which has supplied volume in it

`--chapter [Integer]` - will download manga which has supplied chapter in it

`--saver` - will download images of lower quality and lower download size; will save network resources and reduce download time

`--stat` - will add txt file which contains status information

`--quiet` - will not use curses window output

`--max-consecutive [Integer]` - will download manga images by supplied number at once; it is highly recommended to use **MAX *50*** (default is *40*) because of lack of performance and non complete manga downloading, meaning chapter will not download correctly, meaning missing pages, **!! USE IT BASED ON YOUR INTERNET SPEED, IF YOU HAVE SLOW INTERNET SPEED USE LOWER NUMBER**

`--force` - will download manga even if it already exists

`--offset [Integer]` - changes start offset e.g. 50 starts from chapter 50

`--database-offset [Integer]` - changes start offset e.g. 50 starts from 50 item in database; this occurs before manga is sorted, which result in some weird behavior like missing chapters; For users using `--unsorted`

`--unsorted` - database will not be sorted

`--cwd` - change current working directory

`--encode` - will print url in program readable format

`--log` - will print log

`--search` - will search for manga by its title

`--web` - will enter web mode and will open browser on port 8080, core lock file will not be initialized; if ctrl+c mid download, program cache will not be automatically cleared, there is button in web to exit program. If program can not be exited with ctrl+c use it to exit program or type "<http://127.0.0.1:8080/end>" in browser, that can happen when you use program without web flag and then again with web flag in same terminal

`--server` - will start server from which you can download manga through local internet

`--music` - will play music during downloading 1. Wushu Dolls, 2. Militech, 3. Musorshchiki

## Subcommands

e.g. `cargo run -r -- app --force-setup` or `mdown app --force-setup`

### app

`--force-setup` - will force all setup procedures

`--delete` - will delete database

`--force-delete` - will force to delete *.lock file which is stopping from running another instance of program; NOTE that if you already have one instance running it will fail to delete the original file and thus it will crash

`--reset` - after confirmation will do factory reset

### database

`--check` - check for for any manga updates

`--update` - will download manga updates

`--show` - will show current manga in database

`--show-all` - will show current chapters in database

`--show-log` - will Shows current logs in database

### settings

`--folder` - will set default folder name; if its left empty then it will remove the default folder

## Help

- There are some function that will work with or without specifying argument e.g. `--music`. You can see it with `--help` flag and if there is \<ARG\> you need to specify argument else if [\<ARG\>] you don't need to specify argument, it will be defaulted

---

Using [yt-dlp](https://github.com/yt-dlp/yt-dlp);

First time configuration is using yt-dlp for downloading some stuff

- If you get message that lock file is present, and you believe you don't have already have program started, use `--force-delete` option to force it to delete lock file

- Will download cover image and description even if it did NOT download any more chapters in currently downloaded files AND if it do NOT find any eligible manga chapters it will delete the original
  - e.g. whole manga was in Japanese and didn't find any English chapters which results in 0 downloads

- Every non-final downloads and temporary files will be put in .cache folder which if empty will be deleted afterwards

- Manga name will be automatically shortened when it exceeds 70 characters
