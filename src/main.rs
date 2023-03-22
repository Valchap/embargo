use toml::{map::Map, Table, Value};

const CONFIG_FILE_NAME: &str = "Embargo.toml";

const COMPILER_KEY: &str = "compiler";
const FLAGS_KEY: &str = "flags";
const DEBUG_FLAGS_KEY: &str = "debug-flags";
const RELEASE_FLAGS_KEY: &str = "release-flags";
const SRC_FOLDER_KEY: &str = "src-folder";
const INCLUDE_FOLDER_KEY: &str = "include-folder";
const BUILD_FOLDER_KEY: &str = "build-folder";

const DEFAULT_COMPILER: &str = "clang++";
const DEFAULT_FLAGS: &str = "-Wall -Wextra -pedantic";
const DEFAULT_DEBUG_FLAGS: &str = "-g";
const DEFAULT_RELEASE_FLAGS: &str = "-O2 -s";
const DEFAULT_SRC_FOLDER: &str = "src";
const DEFAULT_INCLUDE_FOLDER: &str = "include";
const DEFAULT_BUILD_FOLDER: &str = "build";

const SEPARATOR: char = std::path::MAIN_SEPARATOR;

struct Configuration {
    compiler: String,

    flags: String,
    debug_flags: String,
    release_flags: String,

    src_folder: String,
    include_folder: String,
    build_folder: String,
}

fn read_string_key(toml: &Map<String, Value>, key_name: &str) -> Result<Option<String>, String> {
    if let Some(value) = toml.get(key_name) {
        if let Some(slice) = value.as_str() {
            Ok(Some(slice.to_string()))
        } else {
            Err(format!("Key {key_name} must be a string"))
        }
    } else {
        Ok(None)
    }
}

fn read_configuration(config_path: &str) -> Result<Configuration, String> {
    match std::fs::read_to_string(format!("{config_path}{SEPARATOR}{CONFIG_FILE_NAME}")) {
        Ok(toml_str) => match toml_str.parse::<Table>() {
            Ok(toml) => {
                let compiler =
                    read_string_key(&toml, COMPILER_KEY)?.unwrap_or(DEFAULT_COMPILER.to_owned());
                let flags = read_string_key(&toml, FLAGS_KEY)?.unwrap_or(DEFAULT_FLAGS.to_owned());
                let debug_flags = read_string_key(&toml, DEBUG_FLAGS_KEY)?
                    .unwrap_or(DEFAULT_DEBUG_FLAGS.to_owned());
                let release_flags = read_string_key(&toml, RELEASE_FLAGS_KEY)?
                    .unwrap_or(DEFAULT_RELEASE_FLAGS.to_owned());
                let src_folder = read_string_key(&toml, SRC_FOLDER_KEY)?
                    .unwrap_or(DEFAULT_SRC_FOLDER.to_owned());
                let include_folder = read_string_key(&toml, INCLUDE_FOLDER_KEY)?
                    .unwrap_or(DEFAULT_INCLUDE_FOLDER.to_owned());
                let build_folder = read_string_key(&toml, BUILD_FOLDER_KEY)?
                    .unwrap_or(DEFAULT_BUILD_FOLDER.to_owned());

                Ok(Configuration {
                    compiler,
                    flags,
                    debug_flags,
                    release_flags,
                    src_folder,
                    include_folder,
                    build_folder,
                })
            }

            Err(_toml_parse_error) => Err(format!(
                "Can't parse {CONFIG_FILE_NAME} file ! Does it contain valid toml ?"
            )),
        },
        Err(_toml_read_error) => Err(format!(
            "Can't read {CONFIG_FILE_NAME} file ! Are you in a project folder ?"
        )),
    }
}

fn config_command(config: &Configuration) {
    println!("Embargo is configured as follow: ");
    println!("    Compiler          {}", config.compiler);
    println!("    Flags             {}", config.flags);
    println!("    Debug flags       {}", config.debug_flags);
    println!("    Release flags     {}", config.release_flags);
    println!("    Src folder        {}", config.src_folder);
    println!("    Include folder    {}", config.include_folder);
    println!("    Build folder      {}", config.build_folder);
}

fn help_command() {
    println!("Usage: embargo [COMMAND]");
    println!("Available commands :");
    println!("    build            Build the app in debug mode");
    println!("    release-build    Build the app in release mode");
    println!("    run              Build the app in debug mode and run it");
    println!("    release-run      Build the app in release mode and run it");
    println!("    config           Show embargo configuration as defined in Embargo.toml file");
    println!("    help             Show this help message")
}

fn main() {
    match read_configuration(".") {
        Ok(config) => {
            if let Some(first_arg) = std::env::args().nth(1) {
                match first_arg.as_str() {
                    "config" => {
                        config_command(&config);
                    }
                    "help" => {
                        help_command();
                    }
                    _ => {
                        println!("Unknown command, try `embargo help` for more informations");
                    }
                }
            } else {
                println!("Embargo takes a command as parameter, try `embargo help` for more informations");
            }
        }
        Err(err_msg) => {
            println!("{err_msg}");
            return;
        }
    }
}
