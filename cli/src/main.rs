use clap::{Arg, ArgAction, ArgMatches, Command};
use crate::command::{emphasize1, err, warn};
pub use crate::config::{Config, Format, Profile};
use std::env;
use std::io::{Error as IOError, ErrorKind, Result as IOResult};
// re-exports entry points.
use std::path::PathBuf;

mod command;
mod config;

// logo generated from http://www.patorjk.com/software/taag/#p=display&h=1&v=3&f=Doom&t=mtklogo
const LOGO: &'static [u8] = include_bytes!("../resources/logo.txt");

fn main() {
    match wrapped_main() {
        Ok(()) => (),
        Err(e) => {
            println!("{}: {}", warn("error"), err(e));
            std::process::exit(1);
        }
    }
}

fn wrapped_main() -> IOResult<()> {
    // defines common args amongst commands.
    let slots_arg = Arg::new("slots")
        .help("Extracts only these slots, other slot remain in raw .z format.")
        .value_name("slots")
        .num_args(1)
        .long("slots")
        .conflicts_with("zip");

    let path_arg = Arg::new("path")
        .help("Path to input `logo.bin`")
        .required(true)
        .index(1);

    let prg = Command::new("mtklogo")
        .version("0.1.2")
        .author("arlept, arnaud@lepoint.net")
        .about("Yet another Android Logo Customizer for MTK devices!\nIt packs or repacks images from an MTK `logo.bin` file.")
        .subcommand(Command::new("unpack")
            .about("Unpacks a logo image")
            .arg(Arg::new("profile")
                .help("Uses an alternative profile name")
                .value_name("profile")
                .short('p')
                .long("profile"))
            .arg(Arg::new("config")
                .help("Uses an alternative configuration file")
                .value_name("configfile")
                .num_args(1)
                .short('c')
                .long("config"))
            .arg(Arg::new("mode")
                .help("Overrides profile's color mode")
                .value_name("mode")
                .short('m')
                .long("mode"))
            .arg(Arg::new("flip")
                .help("Flips orientation")
                .short('f')
                .long("flip")
                .action(ArgAction::SetTrue))
            .arg(Arg::new("zip")
                .help("Do not convert to png, extract as plain .z file")
                .short('z')
                .long("zip")
                .action(ArgAction::SetTrue)
                .conflicts_with("slots"))
            .arg(Arg::new("output")
                .help("Sets images output path")
                .value_name("output")
                .num_args(1)
                .short('o')
                .long("output"))
            .arg(Arg::new("no-out")
                .help("Do not extract images, just checks image formats.")
                .short('n')
                .long("no-out")
                .action(ArgAction::SetTrue)
                .conflicts_with("output"))
            .arg(path_arg.clone())
            .arg(slots_arg.clone())
        )

        .subcommand(Command::new("explore")
            .about("Unpacks a logo image with the specified format\n\
this is useful is you don't know the image format, you'll probably find out.")
            .arg(Arg::new("output")
                .help("Sets images output directory")
                .value_name("output")
                .num_args(1)
                .short('o')
                .long("output"))
            .arg(Arg::new("width")
                .help("Image width in pixels")
                .value_name("width")
                .required(true)
                .num_args(1)
                .short('w')
                .long("width"))
            .arg(path_arg.clone())
            .arg(slots_arg.clone())
        )

        .subcommand(Command::new("guess")
            .about("Tries to guess an image dimension knowing its buffer size.\n\
Note: the program may be very slow if your input size is a large prime number!")
            .arg(Arg::new("size")
                .help("Image size in bytes")
                .value_name("size")
                .required(true)
                .num_args(1)
                .short('s')
                .long("size"))
        )

        .subcommand(Command::new("repack")
            .about("Repacks a logo image")
            .arg(Arg::new("output")
                .value_name("output")
                .help("Path to output `logo.bin`")
                .required(true)
                .num_args(1)
                .short('o')
                .long("output"))
            .arg(Arg::new("files")
                .help("Files to repack. Take care of specifying the exact set of files!")
                .value_name("files")
                .num_args(1..)
                .required(true))
            .arg(Arg::new("alpha")
                .help("Strips Alpha channel, assume image is opaque")
                .short('a')
                .long("alpha")
                .action(ArgAction::SetTrue))
        )
    ;
    let matches = prg.get_matches();

    println!("{}", emphasize1(String::from_utf8_lossy(LOGO)));

    if let Some(matches) = matches.subcommand_matches("unpack") {
        let config = solve_config(matches)?;
        let profile = matches.get_one::<String>("profile").map_or("default", String::as_str);
        let mode = matches.get_one::<String>("mode").map(String::as_str);
        let flip = matches.get_flag("flip");
        let zip = matches.get_flag("zip");
        let check = matches.get_flag("no-out");
        let path = solve_path(matches)?;
        let output = solve_output(matches)?;
        let slots = solve_slots(matches)?;

        command::run_unpack(config, slots, profile, mode, flip, zip, check, path, output)
    } else if let Some(matches) = matches.subcommand_matches("explore") {
        let path = solve_path(matches)?;
        let output = solve_output(matches)?;
        let width = parse_or_error::<u32>(matches, "width")?;
        let slots = solve_slots(matches)?;
        command::run_explore(path, slots, output, width)
    } else if let Some(matches) = matches.subcommand_matches("repack") {
        let maybe_files = matches.get_many::<String>("files")
            .map(|vals| vals.collect::<Vec<_>>());
        let files = maybe_files.map_or_else(
            || Err(IOError::new(ErrorKind::Other, "no files to convert")),
            |f| Ok(f))?;
        let paths = files
            .iter()
            .map(|f| PathBuf::from(f.as_str()))
            .collect();
        let output = matches.get_one::<String>("output")
            .map(|o| PathBuf::from(o.as_str()))
            .unwrap_or(PathBuf::default());
        let strip_alpha = matches.get_flag("alpha");
        command::run_repack(output, paths, strip_alpha)
    } else if let Some(matches) = matches.subcommand_matches("guess") {
        let size = parse_or_error::<usize>(matches, "size")?;
        command::run_guess(size)
    } else {
        println!("Use --help for usage.");
        Err(IOError::new(ErrorKind::InvalidInput, "unrecognized command arguments."))
    }
}

fn value_or_error(matches: &ArgMatches, label: &str) -> IOResult<String> {
    matches.get_one::<String>(label).map_or_else(
        || Err(IOError::new(ErrorKind::InvalidInput, format!("'{}' unspecified.", label))),
        |v| Ok(v.clone()))
}

fn parse_or_error<T>(matches: &ArgMatches, label: &str) -> IOResult<T>
    where T: std::str::FromStr {
    value_or_error(matches, label)
        .and_then(|v| v.parse::<T>().map_err(|_| IOError::new(
            ErrorKind::InvalidInput, format!("'{}' has not expected format", label))))
}


fn solve_output(matches: &ArgMatches) -> IOResult<PathBuf> {
    let output = value_or_error(matches, "output")
        .map(PathBuf::from)
        .or_else(|_| env::current_dir())?;
    ensure_existing_directory(&output)?;
    Ok(output)
}

fn solve_config(matches: &ArgMatches) -> IOResult<Config> {
    match matches.get_one::<String>("config") {
        Some(c) => {
            let path = PathBuf::from(c);
            ensure_existing_file(&path)?;
            Config::from_file(path.as_path())
        }
        None => Config::load()
    }
}

fn solve_path(matches: &ArgMatches) -> IOResult<PathBuf> {
    let path = value_or_error(matches, "path").map(PathBuf::from)?;
    ensure_existing_file(&path)?;
    Ok(path)
}

fn solve_slots(matches: &ArgMatches) -> IOResult<Option<Vec<usize>>> {
    match matches.get_one::<String>("slots") {
        Some(slots) => {
            let tokens: Vec<&str> = slots.split(',').collect();
            let mut sizes: Vec<usize> = Vec::with_capacity(tokens.len());
            for s in tokens.iter() {
                let value = s.parse::<usize>()
                    .map_err(|_| IOError::new(
                        ErrorKind::InvalidInput, format!("'{}' is not an integer", s)))?;
                sizes.push(value);
            }
            Ok(Some(sizes))
        }
        None => Ok(None)
    }
}

fn ensure_existing_file(path: &PathBuf) -> IOResult<()> {
    if path.exists() {
        Ok(())
    } else {
        Err(IOError::new(ErrorKind::InvalidInput, format!("{} must be an existing file.", path.display())))
    }
}

fn ensure_existing_directory(path: &PathBuf) -> IOResult<()> {
    if path.exists() && path.is_dir() {
        Ok(())
    } else {
        Err(IOError::new(ErrorKind::InvalidInput, format!("{} must be an existing directory.", path.display())))
    }
}
