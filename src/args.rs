use clap::{ ArgGroup, Parser, Subcommand };
use lazy_static::lazy_static;
use parking_lot::Mutex;

lazy_static! {
    pub(crate) static ref ARGS: Mutex<Args> = Mutex::new(Args::from_args());
    pub(crate) static ref ARGS_CHECK: bool = ARGS.lock().check.clone();
    pub(crate) static ref ARGS_UPDATE: bool = ARGS.lock().update.clone();
    pub(crate) static ref ARGS_QUIET: bool = ARGS.lock().quiet.clone();
    pub(crate) static ref ARGS_LOG: bool = ARGS.lock().log.clone();
    pub(crate) static ref ARGS_ENCODE: String = ARGS.lock().encode.clone();
    pub(crate) static ref ARGS_DEV: bool = ARGS.lock().dev.clone();
    pub(crate) static ref ARGS_CWD: String = ARGS.lock().cwd.clone();
    pub(crate) static ref ARGS_UNSORTED: bool = ARGS.lock().unsorted.clone();
    pub(crate) static ref ARGS_SHOW: Option<Option<String>> = ARGS.lock().show.clone();
    pub(crate) static ref ARGS_DEBUG: bool = ARGS.lock().debug.clone();
    pub(crate) static ref ARGS_SHOW_ALL: bool = ARGS.lock().show_all.clone();
    pub(crate) static ref ARGS_SHOW_LOG: bool = ARGS.lock().show_log.clone();
    pub(crate) static ref ARGS_WEB: bool = ARGS.lock().web.clone();
    pub(crate) static ref ARGS_GUI: bool = ARGS.lock().gui.clone();
    pub(crate) static ref ARGS_SERVER: bool = ARGS.lock().server.clone();
    pub(crate) static ref ARGS_RESET: bool = match ARGS.lock().subcommands {
        Some(Commands::App { reset, .. }) => reset,
        Some(_) => false,
        None => false,
    };
    pub(crate) static ref ARGS_DELETE: bool = match ARGS.lock().subcommands {
        Some(Commands::App { delete, .. }) => delete,
        Some(_) => false,
        None => false,
    };
    pub(crate) static ref ARGS_FORCE_DELETE: bool = match ARGS.lock().subcommands {
        Some(Commands::App { force_delete, .. }) => force_delete,
        Some(_) => false,
        None => false,
    };
    pub(crate) static ref ARGS_FORCE_SETUP: bool = match ARGS.lock().subcommands {
        Some(Commands::App { force_setup, .. }) => force_setup,
        Some(_) => false,
        None => false,
    };
}

/// Mangadex Manga downloader
#[derive(Parser)]
#[command(
    author = "GrenManSK",
    version,
    about,
    help_template = "{before-help}{name} ({version}) - {author}

{about}

{usage-heading} {usage}

{all-args}
{after-help}",
    help_expected = true,
    long_about = None,
    after_help = "Thanks for using Mdown"
)]
#[clap(group = ArgGroup::new("Search-Options").args(&["url", "search"]))]
#[clap(group = ArgGroup::new("Mod-Options").args(&["web", "server", "gui", "encode"]))]
pub(crate) struct ParserArgs {
    #[arg(
        short,
        long,
        value_name = "SITE",
        default_value_t = String::from("UNSPECIFIED"),
        next_line_help = true,
        help = format!(
            "url of manga, supply in the format of https:/{}",
            "/mangadex.org/title/[id]/\nor UUID\n"
        )
        // Reason for this format!() is because highlighting error in VS Code;
        // precisely "//" this will break it "url of manga, supply in the format of https://mangadex.org/title/[id]/"
    )] pub(crate) url: String,
    #[arg(
        short,
        long,
        value_name = "LANGUAGE",
        default_value_t = String::from("en"),
        next_line_help = true,
        help = "language of manga to download; \"*\" is for all languages\n"
    )] pub(crate) lang: String,
    #[arg(
        short,
        long,
        default_value_t = String::from("*"),
        next_line_help = true,
        help = "name of the manga\n"
    )] pub(crate) title: String,
    #[arg(
        short,
        long,
        default_value_t = String::from("."),
        next_line_help = true,
        help = "put all chapters in folder specified,\n- if folder name is name it will put in folder same as manga name\n- if folder name is name and title is specified it will make folder same as title\n"
    )] pub(crate) folder: String,
    #[arg(
        short,
        long,
        default_value_t = String::from("*"),
        next_line_help = true,
        help = "download only specified volume\n"
    )] pub(crate) volume: String,
    #[arg(
        short,
        long,
        default_value_t = String::from("*"),
        next_line_help = true,
        help = "download only specified chapter\n"
    )] pub(crate) chapter: String,
    ///
    #[arg(
        short,
        long,
        next_line_help = true,
        help = "download images of lower quality and lower download size; will save network resources and reduce download time"
    )]
    pub(crate) saver: bool,
    #[arg(
        long,
        next_line_help = true,
        help = "add markdown file which contains status information"
    )] pub(crate) stat: bool,
    #[arg(long, next_line_help = true, help = "Won't use curses window")] pub(crate) quiet: bool,
    #[arg(
        short,
        long,
        default_value_t = String::from("40"),
        next_line_help = true,
        help = "download manga images by supplied number at once;\nit is highly recommended to use MAX 50 because of lack of performance and non complete manga downloading,\nmeaning chapter will not download correctly, meaning missing or corrupt pages\n"
    )] pub(crate) max_consecutive: String,
    #[arg(
        long,
        next_line_help = true,
        help = "download manga even if it already exists"
    )] pub(crate) force: bool,
    #[arg(
        short,
        long,
        default_value_t = String::from("0"),
        next_line_help = true,
        help = "changes start offset e.g. 50 starts from chapter 50,\nalthough if manga contains chapter like 3.1, 3.2 starting chapter will be moved by number of these chapters\n"
    )] pub(crate) offset: String,
    #[arg(
        short,
        long,
        default_value_t = String::from("0"),
        next_line_help = true,
        help = "changes start offset e.g. 50 starts from 50 item in database;\nthis occurs before manga is sorted, which result in some weird behavior like missing chapters\n"
    )] pub(crate) database_offset: String,
    #[arg(
        long,
        next_line_help = true,
        help = "database will not be sorted"
    )] pub(crate) unsorted: bool,
    #[arg(
        long,
        default_value_t = String::from("./"),
        next_line_help = true,
        help = "change current working directory\n"
    )] pub(crate) cwd: String,
    #[arg(
        short,
        long,
        next_line_help = true,
        default_value_t = String::from(""),
        help = "print url in program readable format\n"
    )] pub(crate) encode: String,
    #[arg(
        long,
        next_line_help = true,
        help = "print log and write it in log,json"
    )] pub(crate) log: bool,
    #[arg(
        long,
        default_value_t = String::from("*"),
        next_line_help = true,
        help = "download manga by manga title\n"
    )] pub(crate) search: String,
    #[arg(
        short,
        long,
        next_line_help = true,
        help = "enter web mode and will open browser on port 8080, core lock file will not be initialized; result will be printed gradually during download process"
    )] pub(crate) web: bool,
    #[arg(long, next_line_help = true, help = "Starts server")] pub(crate) server: bool,
    /// Reset-Options
    #[arg(long, next_line_help = true, help = "Gui version of mdown")]
    pub(crate) gui: bool,
    /// dev
    #[arg(long, next_line_help = true, help = "debug")]
    pub(crate) debug: bool,
    #[arg(long, next_line_help = true, help = "dev")] pub(crate) dev: bool,
    #[command(subcommand)]
    pub(crate) subcommands: Option<Commands>,
}

#[derive(Subcommand, Clone, Debug)]
pub(crate) enum Commands {
    Database {
        #[arg(long, next_line_help = true, help = "Check downloaded files for errors")] check: bool,
        #[arg(
            long,
            next_line_help = true,
            help = "Check downloaded files for errors"
        )] update: bool,
        #[arg(
            long,
            next_line_help = true,
            help = "Shows current manga in database"
        )] show: Option<Option<String>>,
        #[arg(
            long,
            next_line_help = true,
            help = "Shows current chapters in database"
        )] show_all: bool,
        #[arg(long, next_line_help = true, help = "Shows current logs in database")] show_log: bool,
    },
    Settings {
        #[arg(
            long,
            next_line_help = true,
            help = "set default name of folder"
        )] folder: Option<Option<String>>,
    },
    App {
        #[arg(long, next_line_help = true, help = "Force first time setup")] force_setup: bool,
        #[arg(
            long,
            next_line_help = true,
            help = "force to delete *.lock file which is stopping from running another instance of program;\nNOTE that if you already have one instance running it will fail to delete the original file and thus it will crash"
        )]
        force_delete: bool,
        #[arg(long, next_line_help = true, help = "Delete dat.json")] delete: bool,
        #[arg(
            long,
            next_line_help = true,
            help = "Delete all files created by program"
        )] reset: bool,
    },
}

impl Default for Commands {
    fn default() -> Self {
        Commands::Database {
            check: false,
            update: false,
            show: None,
            show_all: false,
            show_log: false,
        }
    }
}

pub(crate) enum Value {
    #[allow(dead_code)] Bool(bool),
    Str(String),
}

pub(crate) struct Args {
    pub(crate) url: String,
    pub(crate) lang: String,
    pub(crate) title: String,
    pub(crate) folder: String,
    pub(crate) volume: String,
    pub(crate) chapter: String,
    pub(crate) saver: bool,
    pub(crate) stat: bool,
    pub(crate) quiet: bool,
    pub(crate) max_consecutive: String,
    pub(crate) force: bool,
    pub(crate) offset: String,
    pub(crate) database_offset: String,
    pub(crate) unsorted: bool,
    pub(crate) cwd: String,
    pub(crate) encode: String,
    pub(crate) log: bool,
    pub(crate) check: bool,
    pub(crate) update: bool,
    pub(crate) search: String,
    pub(crate) show: Option<Option<String>>,
    pub(crate) show_all: bool,
    pub(crate) show_log: bool,
    pub(crate) web: bool,
    pub(crate) server: bool,
    pub(crate) gui: bool,
    pub(crate) debug: bool,
    pub(crate) dev: bool,
    pub(crate) subcommands: Option<Commands>,
}

impl Args {
    pub(crate) fn change(&mut self, typ: &str, to: Value) {
        match (typ, to) {
            ("folder", Value::Str(value)) => {
                self.folder = value;
            }
            _ => (),
        }
    }

    pub(crate) fn from_args() -> Args {
        let args = ParserArgs::parse();
        let subcommands = match args.subcommands {
            Some(ref value) => value,
            None => &Commands::default(),
        };
        Args {
            url: args.url,
            lang: args.lang,
            title: args.title,
            folder: args.folder,
            volume: args.volume,
            chapter: args.chapter,
            saver: args.saver,
            stat: args.stat,
            quiet: args.quiet,
            max_consecutive: args.max_consecutive,
            force: args.force,
            offset: args.offset,
            database_offset: args.database_offset,
            unsorted: args.unsorted,
            cwd: args.cwd,
            encode: args.encode,
            log: args.log,
            check: match subcommands {
                Commands::Database { check, .. } => *check,
                _ => false,
            },
            update: match subcommands {
                Commands::Database { update, .. } => *update,
                _ => false,
            },
            show: match subcommands {
                Commands::Database { show, .. } => show.clone(),
                _ => None,
            },
            show_all: match subcommands {
                Commands::Database { show_all, .. } => *show_all,
                _ => false,
            },
            show_log: match subcommands {
                Commands::Database { show_log, .. } => *show_log,
                _ => false,
            },
            web: args.web,
            server: args.server,
            search: args.search,
            gui: args.gui,
            debug: args.debug,
            dev: args.dev,
            subcommands: args.subcommands,
        }
    }

    #[cfg(feature = "gui")]
    pub(crate) fn from(
        url: String,
        lang: String,
        title: String,
        folder: String,
        volume: String,
        chapter: String,
        saver: bool,
        stat: bool,
        max_consecutive: String,
        force: bool,
        offset: String,
        database_offset: String
    ) -> Args {
        Args {
            url: url,
            lang: lang,
            title: title,
            folder: folder,
            volume: volume,
            chapter: chapter,
            saver: saver,
            stat: stat,
            quiet: *ARGS_QUIET,
            max_consecutive: max_consecutive,
            force: force,
            offset: offset,
            database_offset: database_offset,
            unsorted: *ARGS_UNSORTED,
            cwd: ARGS_CWD.to_string(),
            encode: ARGS_ENCODE.to_string(),
            log: *ARGS_LOG,
            check: *ARGS_CHECK,
            update: *ARGS_UPDATE,
            show: ARGS_SHOW.clone(),
            show_all: *ARGS_SHOW_ALL,
            show_log: *ARGS_SHOW_LOG,
            web: *ARGS_WEB,
            server: *ARGS_SERVER,
            search: String::new(),
            gui: *ARGS_GUI,
            debug: *ARGS_DEBUG,
            dev: *ARGS_DEV,
            subcommands: ARGS.lock().subcommands.clone(),
        }
    }
}
