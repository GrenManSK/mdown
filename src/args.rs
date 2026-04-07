use clap::{ ArgGroup, Parser, Subcommand };
use lazy_static::lazy_static;
use parking_lot::Mutex;

use crate::metadata::Settings;

const MAX_CONSECUTIVE: usize = 40;
const DEFAULT_LANG: &str = "en";
pub(crate) const ARGS_UNSPECIFIED: &str = "UNSPECIFIED";

lazy_static! {
    /// A globally accessible, thread-safe instance of the parsed command-line arguments.
    ///
    /// This instance is protected by a `Mutex` to allow safe concurrent access from multiple threads.
    pub(crate) static ref ARGS: Mutex<Args> = Mutex::new(Args::from_args());

    /// Indicates whether the `quiet` mode is enabled.
    pub(crate) static ref ARGS_INFO: String = ARGS.lock().info.clone();

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
    pub(crate) static ref ARGS_CWD: String = {
        let mut cwd = ARGS.lock().cwd.clone();
        if !cwd.ends_with('/') {
            cwd.push('/');
        }
        cwd
    };

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

/// MangaDex manga downloader
#[derive(Parser)]
#[command(
    author = "GrenManSK",
    version,
    about = "Download manga from MangaDex",
    help_template = "\x1b[1m{name}\x1b[0m {version} - {author}\n{about}\n\n\x1b[1mUSAGE:\x1b[0m\n  {usage}\n\n\x1b[1mEXAMPLES:\x1b[0m\n  mdown --url https://mangadex.org/title/UUID\n  mdown --url UUID --lang en --saver\n  mdown --search \"One Piece\"\n  mdown --web\n\n\x1b[1mOPTIONS:\x1b[0m\n{all-args}\n\n{after-help}",
    help_expected = true,
    long_about = "A manga downloader for MangaDex. Supports batch downloading, web reader,\ndesktop GUI, and LAN server. Download chapters as .cbz files with metadata.\n\nData from MangaDex (mangadex.org). Chapters by scanlation groups.",
    after_help = "\x1b[1mSUBCOMMANDS:\x1b[0m\n  app          Application management (setup, reset, backup, update)\n  database     Database operations (check updates, show library)\n  settings     Configure defaults (folder, stat, backup, music)\n\nRun 'mdown <subcommand> --help' for more info on a subcommand.\n\n\x1b[1mNOTES:\x1b[0m\n  - First run shows a tutorial; use --skip-tutorial to disable\n  - Downloads go to current directory; use --folder to change\n  - Use --saver for faster downloads (lower quality images)\n  - Lock file prevents multiple instances; use 'app --force-delete' if stuck"
)]
#[clap(group = ArgGroup::new("Search-Options").args(&["url", "search"]))]
#[clap(group = ArgGroup::new("Mod-Options").args(&["web", "server", "gui", "encode"]))]
#[clap(group = ArgGroup::new("Tutorial-Options").args(&["tutorial", "skip_tutorial"]))]
pub(crate) struct ParserArgs {
    /// Manga URL or UUID
    #[arg(
        short,
        long,
        value_name = "URL",
        default_value_t = String::from(ARGS_UNSPECIFIED),
        help = "MangaDex URL or manga UUID\n  Example: https://mangadex.org/title/a1c7c817-... or just the UUID"
    )]
    pub(crate) url: String,

    /// Show manga info without downloading
    #[arg(
        short,
        long,
        value_name = "URL",
        default_value_t = String::from(ARGS_UNSPECIFIED),
        help = "Show manga/chapter info without downloading\n  Same format as --url"
    )]
    pub(crate) info: String,

    /// Language code (default: en)
    #[arg(
        short,
        long,
        value_name = "CODE",
        default_value_t = String::from(DEFAULT_LANG),
        help = "Language to download. Use '*' for all languages\n  Common: en, es, ja, zh, ko, fr, de, pt-br, ru\n  See: https://api.mangadex.org/docs/3-enumerations/#language-codes"
    )]
    pub(crate) lang: String,

    /// Custom title for the manga
    #[arg(
        short,
        long,
        value_name = "NAME",
        default_value_t = String::from("*"),
        help = "Custom name for downloaded manga (used in filenames)"
    )]
    pub(crate) title: String,

    /// Output folder
    #[arg(
        short,
        long,
        value_name = "PATH",
        default_value_t = String::from("."),
        help = "Folder to save downloads\n  Use 'name' to auto-name folder after manga\n  Example: --folder \"My Manga\" or --folder \"name\""
    )]
    pub(crate) folder: String,

    /// Download specific volume
    #[arg(
        short,
        long,
        value_name = "NUM",
        default_value_t = String::from("*"),
        help = "Only download chapters from this volume\n  Example: --volume 1"
    )]
    pub(crate) volume: String,

    /// Download specific chapter
    #[arg(
        short,
        long,
        value_name = "NUM",
        default_value_t = String::from("*"),
        help = "Only download this chapter number\n  Example: --chapter 5 or --chapter 10.5"
    )]
    pub(crate) chapter: String,

    /// Use data-saver images
    #[arg(short, long, help = "Download smaller/compressed images (faster, less bandwidth)")]
    pub(crate) saver: bool,

    /// Generate download statistics
    #[arg(long, help = "Create a .txt file with download statistics")]
    pub(crate) stat: bool,

    /// Quiet mode (no terminal UI)
    #[arg(long, help = "Disable curses terminal output")]
    pub(crate) quiet: bool,

    /// Max parallel image downloads
    #[arg(
        short,
        long,
        value_name = "N",
        default_value_t = MAX_CONSECUTIVE,
        help = "Images to download simultaneously (default: 40, max recommended: 50)\n  Lower values = slower but more stable\n  Use lower on slow connections"
    )]
    pub(crate) max_consecutive: usize,

    /// Force re-download existing files
    #[arg(long, help = "Re-download chapters even if they already exist")]
    pub(crate) force: bool,

    /// Skip first N chapters
    #[arg(
        short,
        long,
        value_name = "N",
        default_value_t = String::from("0"),
        help = "Skip the first N chapters\n  Example: --offset 10 skips chapters 1-10"
    )]
    pub(crate) offset: String,

    /// Database query offset
    #[arg(
        short,
        long,
        value_name = "N",
        default_value_t = String::from("0"),
        help = "Start from Nth item in database (before sorting)\n  May cause missing chapters with default sorting"
    )]
    pub(crate) database_offset: String,

    /// Don't sort chapters
    #[arg(long, help = "Keep original chapter order (don't sort)")]
    pub(crate) unsorted: bool,

    /// Working directory
    #[arg(
        long,
        value_name = "PATH",
        default_value_t = String::from("./"),
        help = "Change the working directory"
    )]
    pub(crate) cwd: String,

    /// Encode URL for scripts
    #[arg(
        short,
        long,
        value_name = "URL",
        default_value_t = String::new(),
        help = "Print URL in encoded format (for scripting)"
    )]
    pub(crate) encode: String,

    /// Enable file logging
    #[arg(long, help = "Write logs to log.json")]
    pub(crate) log: bool,

    /// Run interactive tutorial
    #[arg(long, help = "Show first-run tutorial")]
    pub(crate) tutorial: bool,

    /// Skip tutorial
    #[arg(long, help = "Skip first-run tutorial")]
    pub(crate) skip_tutorial: bool,

    /// Search by title
    #[arg(
        long,
        value_name = "TITLE",
        default_value_t = String::from("*"),
        help = "Search manga by name instead of URL\n  Example: --search \"One Piece\""
    )]
    pub(crate) search: String,

    /// Play background music
    #[arg(
        long,
        value_name = "[NUM]",
        help = "Play music during downloads (requires 'music' feature)\n  Tracks: 1=Wushu Dolls, 2=Militech, 3=Forgive Me, 4=Valentinos, 5=Force Projection\n  Use --music 2 to pick track 2, or --music for default (1)"
    )]
    pub(crate) music: Option<Option<String>>,

    /// Web reader interface
    #[arg(
        short,
        long,
        help = "Open web reader in browser (localhost:8080)\n  Browse and read downloaded manga"
    )]
    pub(crate) web: bool,

    /// LAN server mode
    #[arg(long, help = "Start a server to share manga on your local network")]
    pub(crate) server: bool,

    /// Desktop GUI
    #[arg(long, help = "Launch graphical interface (requires 'gui' feature)")]
    pub(crate) gui: bool,

    /// Debug output
    #[arg(long, hide = true, help = "Print debug messages")]
    pub(crate) debug: bool,

    /// Debug to file
    #[arg(long, hide = true, help = "Write debug messages to debug.log")]
    pub(crate) debug_file: bool,

    /// Developer mode
    #[arg(long, hide = true, help = "Enable developer features")]
    pub(crate) dev: bool,

    /// Subcommands
    #[command(subcommand)]
    pub(crate) subcommands: Option<Commands>,
}

/// Available subcommands
#[derive(Subcommand, Clone, Debug, PartialEq)]
pub(crate) enum Commands {
    /// Manage your manga database
    ///
    /// Check for updates, view your library, and manage backups.
    ///
    /// Examples:
    ///   mdown database --check
    ///   mdown database --show
    ///   mdown database --update
    Database {
        /// Check for manga updates
        #[arg(long, help = "Check if downloaded manga has new chapters")]
        check: bool,

        /// Download available updates
        #[arg(long, help = "Check for and download manga updates")]
        update: bool,

        /// Show downloaded manga
        #[arg(
            long,
            help = "List manga in your library\n  Optionally provide manga UUID to show details"
        )]
        show: Option<Option<String>>,

        /// Show all chapters
        #[arg(
            long,
            help = "List all downloaded chapters\n  Optionally provide manga UUID to filter"
        )]
        show_all: Option<Option<String>>,

        /// Show download logs
        #[arg(long, help = "View download history logs")]
        show_log: bool,

        /// Show current settings
        #[arg(long, help = "Display saved configuration")]
        show_settings: bool,

        /// Restore from backup
        #[arg(long, help = "Choose a backup file to restore")]
        backup_choose: bool,
    },

    /// Configure application defaults
    ///
    /// Set default folder, enable statistics, configure backup, etc.
    ///
    /// Examples:
    ///   mdown settings --folder "My Manga"
    ///   mdown settings --stat 1
    ///   mdown settings --backup 1
    ///   mdown settings --clear
    Settings {
        /// Set default download folder
        #[arg(long, help = "Set default folder for downloads\n  No value = remove setting")]
        folder: Option<Option<String>>,

        /// Auto-enable statistics
        #[arg(
            long,
            help = "Enable/disable --stat by default\n  Use: 1 (yes), 0 (no), empty (remove)"
        )]
        stat: Option<Option<String>>,

        /// Auto-backup
        #[arg(
            long,
            help = "Enable/disable automatic backups\n  Use: 1 (yes), 0 (no), empty (remove)\n  Default: enabled"
        )]
        backup: Option<Option<String>>,

        /// Set default music
        #[arg(
            long,
            help = "Set default music track (requires 'music' feature)\n  Use: 1-5 (track number), empty (remove)"
        )]
        music: Option<Option<String>>,

        /// Clear all settings
        #[arg(long, help = "Remove all saved settings")]
        clear: bool,
    },

    /// Application management
    ///
    /// Setup, reset, backup, and update the application.
    ///
    /// Examples:
    ///   mdown app --force-setup
    ///   mdown app --reset
    ///   mdown app --update
    App {
        /// Re-run first-time setup
        #[arg(long, help = "Force the initial setup wizard to run again")]
        force_setup: bool,

        /// Remove stuck lock file
        #[arg(
            long,
            help = "Delete the .lock file (for when app crashed)\n  WARNING: Don't use if another instance is running"
        )]
        force_delete: bool,

        /// Delete manga database
        #[arg(long, help = "Delete dat.json (manga metadata)")]
        delete: bool,

        /// Factory reset
        #[arg(long, help = "Delete all files and reset to defaults (asks for confirmation)")]
        reset: bool,

        /// Create backup
        #[arg(long, help = "Manually trigger a backup")]
        backup: bool,

        /// Update mdown
        #[arg(long, help = "Update to the latest version")]
        update: bool,
    },
    Default,
}

/// Enum for different types of values used in the application.
pub(crate) enum ArgValue {
    /// A boolean value.
    Bool(bool),

    /// A string value.
    Str(String),

    #[cfg(feature = "music")]
    /// A option option string value used.
    OptOptStr(Option<Option<String>>),
}

/// Structure representing the parsed command-line arguments.
#[derive(PartialEq)]
pub(crate) struct Args {
    pub(crate) url: String,
    pub(crate) info: String,
    pub(crate) lang: String,
    pub(crate) title: String,
    pub(crate) folder: String,
    pub(crate) volume: String,
    pub(crate) chapter: String,
    pub(crate) saver: bool,
    pub(crate) stat: bool,
    pub(crate) quiet: bool,
    pub(crate) max_consecutive: usize,
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
    /// Updates the configuration based on the provided type and value.
    ///
    /// # Parameters
    ///
    /// - `typ: &str` - The type of setting to modify. Supported values:
    ///   - `"folder"`: Updates the folder path if it is set to `"."`.
    ///   - `"stat"`: Updates the `stat` boolean flag.
    ///   - `"backup"`: Updates the `backup` boolean flag.
    ///   - `"music"` (only when the `"music"` feature is enabled): Updates the `music` optional string setting.
    /// - `to: ArgValue` - The new value to assign. Expected variants:
    ///   - `ArgValue::Str(value)`: Used for `"folder"`.
    ///   - `ArgValue::Bool(value)`: Used for `"stat"` and `"backup"`.
    ///   - `ArgValue::OptOptStr(value)`: Used for `"music"` when the `"music"` feature is enabled.
    ///
    /// # Behavior
    ///
    /// - If `typ` is `"folder"` and `self.folder` is `"."`, it updates `self.folder` to `value`.
    /// - If `typ` is `"stat"`, it updates `self.stat` to the provided boolean value.
    /// - If `typ` is `"backup"`, it updates `self.backup` to the provided boolean value.
    /// - If `typ` is `"music"` and the `"music"` feature is enabled, it updates `self.music` to `value.clone()`.
    /// - If `typ` does not match any of the expected values, the function does nothing.
    pub(crate) fn change(&mut self, typ: &str, to: ArgValue) {
        match (typ, to) {
            ("folder", ArgValue::Str(value)) => {
                if self.folder != "." {
                    return;
                }
                self.folder = value;
            }
            ("stat", ArgValue::Bool(value)) => {
                self.stat = value;
            }
            ("backup", ArgValue::Bool(value)) => {
                self.backup = value;
            }
            #[cfg(feature = "music")]
            ("music", ArgValue::OptOptStr(value)) => {
                self.music = value.clone();
            }
            (_, _) => (),
        }
    }

    /// Updates the application settings based on the provided `Settings` struct.
    ///
    /// This function modifies the internal configuration by changing specific settings:
    /// - `"folder"`: Updates the folder path where downloaded manga is stored.
    /// - `"stat"`: Enables or disables statistics tracking.
    /// - `"backup"`: Enables or disables backup functionality.
    /// - `"music"` (*only if the `music` feature is enabled*): Sets the optional music setting.
    ///
    /// # Parameters
    /// - `settings`: A `Settings` struct containing the new configuration values.
    ///
    /// # Example
    /// ```
    /// let mut args = Args::from_args();
    /// let new_settings = Settings {
    ///     folder: String::from("new_folder"),
    ///     stat: true,
    ///     backup: false,
    ///     #[cfg(feature = "music")]
    ///     music: Some(String::from("music_folder")),
    /// };
    /// args.change_settings(new_settings);
    /// ```
    pub(crate) fn change_settings(&mut self, settings: Settings) {
        self.change("folder", ArgValue::Str(settings.folder));
        self.change("stat", ArgValue::Bool(settings.stat));
        self.change("backup", ArgValue::Bool(settings.backup));
        #[cfg(feature = "music")]
        self.change("music", ArgValue::OptOptStr(settings.music));
    }

    /// Parses command-line arguments and constructs an `Args` instance.
    ///
    /// This function utilizes `clap` to parse user-provided arguments and initializes the `Args` struct
    /// accordingly. It extracts values from `ParserArgs` and determines subcommands where applicable.
    ///
    /// # Returns
    /// - An `Args` struct containing all parsed command-line options.
    ///
    /// # Behavior
    /// - If no subcommand is provided, it defaults to `Commands::Default`.
    /// - Fields are assigned directly from `ParserArgs`, with specific handling for database and app subcommands.
    ///
    /// # Example
    /// ```
    /// let args = Args::from_args();
    /// println!("{:?}", args);
    /// ```
    pub(crate) fn from_args() -> Args {
        let args = ParserArgs::parse();
        let subcommands = match args.subcommands {
            Some(ref value) => value,
            None => &Commands::Default,
        };
        Args {
            url: args.url,
            info: args.info,
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

    /// Creates an `Args` instance with specified values, primarily for GUI usage.
    ///
    /// This function is available only when the `gui` feature is enabled (`#[cfg(feature = "gui")]`).
    /// It allows the creation of an `Args` instance by providing explicit values instead of parsing
    /// command-line arguments. Some values are taken from global `ARGS_*` constants or `lazy_static`
    /// variables to maintain synchronization with application settings.
    ///
    /// # Parameters
    /// - `url`: The URL of the manga to download.
    /// - `lang`: The language code for the manga.
    /// - `title`: The title of the manga.
    /// - `folder`: The target folder for downloads.
    /// - `volume`: The volume number (as a string).
    /// - `chapter`: The chapter number (as a string).
    /// - `saver`: Whether to enable the saver mode.
    /// - `stat`: Whether to track statistics.
    /// - `max_consecutive`: The maximum number of consecutive downloads allowed.
    /// - `force`: Whether to force downloads.
    /// - `offset`: The chapter offset for downloads.
    /// - `database_offset`: The offset used for database queries.
    ///
    /// # Behavior
    /// - Uses globally synchronized `ARGS_*` variables for settings not explicitly provided.
    /// - Ensures GUI-related options remain consistent with other parts of the application.
    /// - `ARGS_MUSIC` is not synchronized with the database.
    ///
    /// # Returns
    /// - A fully initialized `Args` struct with GUI-compatible values.
    ///
    /// # Example
    /// ```
    /// let args = Args::from(
    ///     "https://mangadex.org/title/123".to_string(),
    ///     "en".to_string(),
    ///     "Example Manga".to_string(),
    ///     "downloads".to_string(),
    ///     "1".to_string(),
    ///     "2".to_string(),
    ///     true,
    ///     false,
    ///     5,
    ///     false,
    ///     "0".to_string(),
    ///     "0".to_string(),
    /// );
    /// ```
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
        max_consecutive: usize,
        force: bool,
        offset: String,
        database_offset: String
    ) -> Args {
        Args {
            url,
            info: ARGS_INFO.clone(),
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
            backup: *ARGS_BACKUP,
            // ARGS_MUSIC is not synchronized with database
            music: ARGS_MUSIC.clone(),
            tutorial: *ARGS_TUTORIAL,
            skip_tutorial: *ARGS_SKIP_TUTORIAL,
            subcommands: ARGS.lock().subcommands.clone(),
        }
    }
}
