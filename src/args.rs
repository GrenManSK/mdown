use clap::{ ArgGroup, Parser, Subcommand };
use lazy_static::lazy_static;
use parking_lot::Mutex;

use crate::metadata::Settings;

const MAX_CONSECUTIVE: &str = "40";
const DEFAULT_LANG: &str = "en";

lazy_static! {
    /// A globally accessible, thread-safe instance of the parsed command-line arguments.
    ///
    /// This instance is protected by a `Mutex` to allow safe concurrent access from multiple threads.
    pub(crate) static ref ARGS: Mutex<Args> = Mutex::new(Args::from_args());

    /// Indicates whether the `check` option is enabled.
    pub(crate) static ref ARGS_CHECK: bool = ARGS.lock().check;

    /// Indicates whether the `update` option is enabled.
    pub(crate) static ref ARGS_UPDATE: bool = ARGS.lock().update;

    /// Indicates whether the `quiet` mode is enabled.
    pub(crate) static ref ARGS_QUIET: bool = ARGS.lock().quiet;

    /// Indicates whether logging is enabled.
    pub(crate) static ref ARGS_LOG: bool = ARGS.lock().log;

    /// The encoding format specified by the user.
    pub(crate) static ref ARGS_ENCODE: String = ARGS.lock().encode.clone();

    /// Indicates whether development mode is enabled.
    pub(crate) static ref ARGS_DEV: bool = ARGS.lock().dev;

    /// The music setting specified by the user, if any.
    pub(crate) static ref ARGS_MUSIC: Option<Option<String>> = ARGS.lock().music.clone();

    /// The current working directory as specified by the user.
    pub(crate) static ref ARGS_CWD: String = ARGS.lock().cwd.clone();

    /// Indicates whether the database sorting is disabled.
    pub(crate) static ref ARGS_UNSORTED: bool = ARGS.lock().unsorted;

    /// The show setting specified by the user, if any.
    pub(crate) static ref ARGS_SHOW: Option<Option<String>> = ARGS.lock().show.clone();

    /// Indicates whether debug mode is enabled.
    pub(crate) static ref ARGS_DEBUG: bool = ARGS.lock().debug;

    /// Indicates whether file-based debug logging is enabled.
    pub(crate) static ref ARGS_DEBUG_FILE: bool = ARGS.lock().debug_file;

    /// The show all setting specified by the user, if any.
    pub(crate) static ref ARGS_SHOW_ALL: Option<Option<String>> = ARGS.lock().show_all.clone();

    /// Show log output is enabled.
    pub(crate) static ref ARGS_SHOW_LOG: bool = ARGS.lock().show_log;

    /// Indicates whether log output is enabled.
    pub(crate) static ref ARGS_SHOW_SETTINGS: bool = ARGS.lock().show_settings;

    /// Indicates whether web mode is enabled.
    pub(crate) static ref ARGS_WEB: bool = ARGS.lock().web;

    /// Indicates whether GUI mode is enabled.
    pub(crate) static ref ARGS_GUI: bool = ARGS.lock().gui;

    /// Indicates whether the server mode is enabled.
    pub(crate) static ref ARGS_SERVER: bool = ARGS.lock().server;

    pub(crate) static ref ARGS_TUTORIAL: bool = ARGS.lock().tutorial;

    pub(crate) static ref ARGS_SKIP_TUTORIAL: bool = ARGS.lock().skip_tutorial;

    /// Indicates whether to reset the application.
    pub(crate) static ref ARGS_RESET: bool = match ARGS.lock().subcommands {
        Some(Commands::App { reset, .. }) => reset,
        Some(_) => false,
        None => false,
    };

    /// Indicates whether to delete application data.
    pub(crate) static ref ARGS_DELETE: bool = match ARGS.lock().subcommands {
        Some(Commands::App { delete, .. }) => delete,
        Some(_) => false,
        None => false,
    };

    /// Indicates whether to force delete application data.
    pub(crate) static ref ARGS_FORCE_DELETE: bool = match ARGS.lock().subcommands {
        Some(Commands::App { force_delete, .. }) => force_delete,
        Some(_) => false,
        None => false,
    };

    /// Indicates whether to force setup the application.
    pub(crate) static ref ARGS_FORCE_SETUP: bool = match ARGS.lock().subcommands {
        Some(Commands::App { force_setup, .. }) => force_setup,
        Some(_) => false,
        None => false,
    };

    /// Indicates whether to force backup.
    pub(crate) static ref ARGS_BACKUP: bool = match ARGS.lock().subcommands {
        Some(Commands::App { backup, .. }) => backup,
        Some(_) => true,
        None => true,
    };

    /// If true program will ask user which backup file to retrieve.
    pub(crate) static ref ARGS_CH_BACKUP: bool = match ARGS.lock().subcommands {
        Some(Commands::Database { backup_choose, .. }) => backup_choose,
        Some(_) => false,
        None => false,
    };
    /// Indicates whether to update app.
    pub(crate) static ref ARGS_APP_UPDATE: bool = match ARGS.lock().subcommands {
        Some(Commands::App { update, .. }) => update,
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
#[clap(group = ArgGroup::new("Tutorial-Options").args(&["tutorial", "skip_tutorial"]))]
pub(crate) struct ParserArgs {
    /// URL of the manga to be downloaded. Provide in the format `https://mangadex.org/title/[id]/` or UUID.
    #[arg(
        short,
        long,
        value_name = "SITE",
        default_value_t = String::from("UNSPECIFIED"),
        next_line_help = true,
        help = "url of manga, supply in the format of https:/mangadex.org/title/[id]/\nor UUID\n"
    )]
    pub(crate) url: String,

    /// Language of the manga to download; "*" is for all languages.
    #[arg(
        short,
        long,
        value_name = "LANGUAGE",
        default_value_t = String::from(DEFAULT_LANG),
        next_line_help = true,
        help = "language of manga to download; \"*\" is for all languages\n"
    )]
    pub(crate) lang: String,

    /// Name of the manga to download.
    #[arg(
        short,
        long,
        default_value_t = String::from("*"),
        next_line_help = true,
        help = "name of the manga\n"
    )]
    pub(crate) title: String,

    /// Folder to save all chapters. If folder name is `name`, it will save in a folder named after the manga. If title is specified, it will create a folder named after the title.
    #[arg(
        short,
        long,
        default_value_t = String::from("."),
        next_line_help = true,
        help = "put all chapters in folder specified,\n- if folder name is name it will put in folder same as manga name\n- if folder name is name and title is specified it will make folder same as title\n"
    )]
    pub(crate) folder: String,

    /// Download only the specified volume.
    #[arg(
        short,
        long,
        default_value_t = String::from("*"),
        next_line_help = true,
        help = "download only specified volume\n"
    )]
    pub(crate) volume: String,

    /// Download only the specified chapter.
    #[arg(
        short,
        long,
        default_value_t = String::from("*"),
        next_line_help = true,
        help = "download only specified chapter\n"
    )]
    pub(crate) chapter: String,

    /// Download images of lower quality and reduce download size.
    #[arg(
        short,
        long,
        next_line_help = true,
        help = "download images of lower quality and lower download size; will save network resources and reduce download time"
    )]
    pub(crate) saver: bool,

    /// Add a markdown file that contains status information.
    #[arg(
        long,
        next_line_help = true,
        help = "add markdown file which contains status information"
    )]
    pub(crate) stat: bool,

    /// Suppress the use of curses window.
    #[arg(long, next_line_help = true, help = "Won't use curses window")]
    pub(crate) quiet: bool,

    /// Number of manga images to download concurrently. Recommended to use a maximum of 50 to avoid performance issues and incomplete downloads.
    #[arg(
        short,
        long,
        default_value_t = String::from(MAX_CONSECUTIVE),
        next_line_help = true,
        help = "download manga images by supplied number at once;\nit is highly recommended to use MAX 50 because of lack of performance and non complete manga downloading,\nmeaning chapter will not download correctly, meaning missing or corrupt pages\n"
    )]
    pub(crate) max_consecutive: String,

    /// Download manga even if it already exists.
    #[arg(long, next_line_help = true, help = "download manga even if it already exists")]
    pub(crate) force: bool,

    /// Start offset for downloading chapters. For example, "50" starts from chapter 50.
    #[arg(
        short,
        long,
        default_value_t = String::from("0"),
        next_line_help = true,
        help = "changes start offset e.g. 50 starts from chapter 50,\nalthough if manga contains chapter like 3.1, 3.2 starting chapter will be moved by number of these chapters\n"
    )]
    pub(crate) offset: String,

    /// Offset in the database; starts from the specified item in the database before sorting.
    #[arg(
        short,
        long,
        default_value_t = String::from("0"),
        next_line_help = true,
        help = "changes start offset e.g. 50 starts from 50 item in database;\nthis occurs before manga is sorted, which result in some weird behavior like missing chapters\n"
    )]
    pub(crate) database_offset: String,

    /// Do not sort the database.
    #[arg(long, next_line_help = true, help = "database will not be sorted")]
    pub(crate) unsorted: bool,

    /// Change the current working directory.
    #[arg(
        long,
        default_value_t = String::from("./"),
        next_line_help = true,
        help = "change current working directory\n"
    )]
    pub(crate) cwd: String,

    /// Print URL in a program-readable format.
    #[arg(
        short,
        long,
        next_line_help = true,
        default_value_t = String::new(),
        help = "print url in program readable format\n"
    )]
    pub(crate) encode: String,

    /// Enable logging and write logs to `log.json`.
    #[arg(long, next_line_help = true, help = "print log and write it in log,json")]
    pub(crate) log: bool,

    #[arg(long, next_line_help = true, help = "will run tutorial")]
    pub(crate) tutorial: bool,

    #[arg(long, next_line_help = true, help = "will not run tutorial")]
    pub(crate) skip_tutorial: bool,

    /// Search for manga by title.
    #[arg(
        long,
        default_value_t = String::from("*"),
        next_line_help = true,
        help = "download manga by manga title\n"
    )]
    pub(crate) search: String,

    /// Play music during downloading. Options include 1. Wushu Dolls, 2. Militech, 3. You Shall Never Have to Forgive Me Again, 4. Valentinos, 5. Force Projection. Default is 1.
    #[arg(
        long,
        next_line_help = true,
        help = "Will play music during downloading\n1. Wushu Dolls\n2. Militech\n3. You Shall Never Have to Forgive Me Again\n4. Valentinos\n5. Force Projection\n[default: 1]"
    )]
    pub(crate) music: Option<Option<String>>,

    /// Enter web mode and open browser on port 8080. The core lock file will not be initialized, and results will be printed gradually during the download process.
    #[arg(
        short,
        long,
        next_line_help = true,
        help = "enter web mode and will open browser on port 8080, core lock file will not be initialized; result will be printed gradually during download process"
    )]
    pub(crate) web: bool,

    /// Start a server mode.
    #[arg(long, next_line_help = true, help = "Starts server")]
    pub(crate) server: bool,

    /// Start a gui mode
    #[arg(long, next_line_help = true, help = "Gui version of mdown")]
    pub(crate) gui: bool,

    /// Development options
    #[arg(long, next_line_help = true, help = "debug")]
    pub(crate) debug: bool,

    #[arg(long, next_line_help = true, help = "debug")]
    pub(crate) debug_file: bool,

    #[arg(long, next_line_help = true, help = "dev")]
    pub(crate) dev: bool,

    /// Subcommands for various application-specific tasks.
    #[command(subcommand)]
    pub(crate) subcommands: Option<Commands>,
}

/// Enum representing the available subcommands for the application.
#[derive(Subcommand, Clone, Debug)]
pub(crate) enum Commands {
    /// Subcommands related to database management.
    Database {
        /// Check downloaded files for errors.
        #[arg(long, next_line_help = true, help = "Check downloaded manga for updates")]
        check: bool,

        /// Update downloaded files.
        #[arg(long, next_line_help = true, help = "Check and downloads files")]
        update: bool,

        /// Show current manga in the database. You can specify an ID to show a particular manga.
        #[arg(
            long,
            next_line_help = true,
            help = "Shows current manga in database; you can put id of manga that you want to show [default: will show all manga in database]"
        )]
        show: Option<Option<String>>,

        /// Show current chapters in the database. You can specify an ID to show a particular chapter.
        #[arg(
            long,
            next_line_help = true,
            help = "Shows current chapters in database; you can put id of manga that you want to show [default: will show all manga in database]"
        )]
        show_all: Option<Option<String>>,

        /// Show current logs in the database.
        #[arg(long, next_line_help = true, help = "Shows current logs in database")]
        show_log: bool,

        /// Shows current settings in database.
        #[arg(long, next_line_help = true, help = "Shows current settings in database")]
        show_settings: bool,

        /// You will choose which backup to retrieve.
        #[arg(long, next_line_help = true, help = "You will choose which backup to retrieve")]
        backup_choose: bool,
    },

    /// Subcommands related to application settings.
    Settings {
        /// Set the default folder name.
        #[arg(
            long,
            next_line_help = true,
            help = "set default name of folder\n[default: Will remove current folder setting]"
        )]
        folder: Option<Option<String>>,
        /// Set if --stat flag should be default.
        #[arg(
            long,
            next_line_help = true,
            help = "set if --stat should be default\n[default: Will remove current folder setting; 1 is for yes, 0 for no]"
        )]
        stat: Option<Option<String>>,
        /// Will backup files
        #[arg(
            long,
            next_line_help = true,
            help = "Will set default of backup files n[default: Will remove current backup setting; 1 is for yes, 0 for no][default for backup is 1]"
        )]
        backup: Option<Option<String>>,
        /// Will start music
        #[arg(
            long,
            next_line_help = true,
            help = "Will play music during downloading\n1. Wushu Dolls\n2. Militech\n3. You Shall Never Have to Forgive Me Again\n4. Valentinos\n5. Force Projection\n[default: Will remove current setting]"
        )]
        music: Option<Option<String>>,

        /// Will remove all settings
        #[arg(long, next_line_help = true, help = "Will remove all settings")]
        clear: bool,
    },

    /// Subcommands related to application management.
    App {
        /// Force the first-time setup.
        #[arg(long, next_line_help = true, help = "Force first time setup")]
        force_setup: bool,

        /// Force delete the `.lock` file, which prevents running another instance.
        #[arg(
            long,
            next_line_help = true,
            help = "force to delete *.lock file which is stopping from running another instance of program;\nNOTE that if you already have one instance running it will fail to delete the original file and thus it will crash"
        )]
        force_delete: bool,

        /// Delete `dat.json`.
        #[arg(long, next_line_help = true, help = "Delete dat.json")]
        delete: bool,

        /// Delete all files created by the program.
        #[arg(long, next_line_help = true, help = "Delete all files created by program")]
        reset: bool,

        /// Will backup files
        #[arg(long, next_line_help = true, help = "Will backup files")]
        backup: bool,

        /// Will update app
        #[arg(long, next_line_help = true, help = "Will update app")]
        update: bool,
    },
    Default,
}

/// Enum for different types of values used in the application.
pub(crate) enum Value {
    /// A boolean value.
    Bool(bool),

    /// A string value.
    Str(String),

    #[cfg(feature = "music")]
    /// A option option string value used.
    OptOptStr(Option<Option<String>>),
}

/// Structure representing the parsed command-line arguments.
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
    pub(crate) tutorial: bool,
    pub(crate) skip_tutorial: bool,
    pub(crate) search: String,
    pub(crate) show: Option<Option<String>>,
    pub(crate) show_all: Option<Option<String>>,
    pub(crate) show_log: bool,
    pub(crate) show_settings: bool,
    pub(crate) web: bool,
    pub(crate) server: bool,
    pub(crate) gui: bool,
    pub(crate) debug: bool,
    pub(crate) debug_file: bool,
    pub(crate) backup: bool,
    pub(crate) dev: bool,
    pub(crate) music: Option<Option<String>>,
    pub(crate) subcommands: Option<Commands>,
}

impl Args {
    /// Updates the value of a specified field in the `Args` struct.
    ///
    /// # Arguments
    ///
    /// * `typ` - The type of value to update.
    /// * `to` - The new value to set.
    pub(crate) fn change(&mut self, typ: &str, to: Value) {
        match (typ, to) {
            ("folder", Value::Str(value)) => {
                if self.folder != "." {
                    return;
                }
                self.folder = value;
            }
            ("stat", Value::Bool(value)) => {
                self.stat = value;
            }
            ("backup", Value::Bool(value)) => {
                self.backup = value;
            }
            #[cfg(feature = "music")]
            ("music", Value::OptOptStr(value)) => {
                self.music = value.clone();
            }
            (_, _) => (),
        }
    }

    pub(crate) fn change_settings(&mut self, settings: Settings) {
        self.change("folder", Value::Str(settings.folder));
        self.change("stat", Value::Bool(settings.stat));
        self.change("backup", Value::Bool(settings.backup));
        #[cfg(feature = "music")]
        self.change("music", Value::OptOptStr(settings.music));
    }

    /// Creates an `Args` instance from the command-line arguments.
    ///
    /// # Returns
    ///
    /// An `Args` instance populated with values from the parsed command-line arguments.
    pub(crate) fn from_args() -> Args {
        let args = ParserArgs::parse();
        let subcommands = match args.subcommands {
            Some(ref value) => value,
            None => &Commands::Default,
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
                Commands::Database { show_all, .. } => show_all.clone(),
                _ => None,
            },
            show_log: match subcommands {
                Commands::Database { show_log, .. } => *show_log,
                _ => false,
            },
            show_settings: match subcommands {
                Commands::Database { show_settings, .. } => *show_settings,
                _ => false,
            },
            backup: match subcommands {
                Commands::App { backup, .. } => *backup,
                _ => false,
            },
            web: args.web,
            server: args.server,
            search: args.search,
            gui: args.gui,
            debug: args.debug,
            debug_file: args.debug_file,
            dev: args.dev,
            music: args.music,
            tutorial: args.tutorial,
            skip_tutorial: args.skip_tutorial,
            subcommands: args.subcommands,
        }
    }

    /// Creates an `Args` instance with default values for GUI mode.
    ///
    /// # Arguments
    ///
    /// * `url` - The URL of the manga.
    /// * `lang` - The language of the manga.
    /// * `title` - The title of the manga.
    /// * `folder` - The folder to store manga chapters.
    /// * `volume` - The volume of the manga.
    /// * `chapter` - The chapter of the manga.
    /// * `saver` - Whether to use the saver mode.
    /// * `stat` - Whether to generate a status file.
    /// * `max_consecutive` - The maximum number of consecutive downloads.
    /// * `force` - Whether to force download.
    /// * `offset` - The start offset for chapters.
    /// * `database_offset` - The start offset for the database.
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
            url,
            lang,
            title,
            folder,
            volume,
            chapter,
            saver,
            stat,
            quiet: *ARGS_QUIET,
            max_consecutive,
            force,
            offset,
            database_offset,
            unsorted: *ARGS_UNSORTED,
            cwd: ARGS_CWD.to_string(),
            encode: ARGS_ENCODE.to_string(),
            log: *ARGS_LOG,
            check: *ARGS_CHECK,
            update: *ARGS_UPDATE,
            show: ARGS_SHOW.clone(),
            show_all: ARGS_SHOW_ALL.clone(),
            show_log: *ARGS_SHOW_LOG,
            show_settings: *ARGS_SHOW_SETTINGS,
            web: *ARGS_WEB,
            server: *ARGS_SERVER,
            search: String::new(),
            gui: *ARGS_GUI,
            debug: *ARGS_DEBUG,
            debug_file: *ARGS_DEBUG_FILE,
            dev: *ARGS_DEV,
            backup: ARGS_BACKUP.clone(),
            // ARGS_MUSIC is not synchronized with database
            music: ARGS_MUSIC.clone(),
            tutorial: *ARGS_TUTORIAL,
            skip_tutorial: *ARGS_SKIP_TUTORIAL,
            subcommands: ARGS.lock().subcommands.clone(),
        }
    }
}
