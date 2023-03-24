use std::process::Command;

use toml::{map::Map, Table, Value};
use walkdir::WalkDir;

const CONFIG_FILE: &str = "Embargo.toml";
const COMPILE_FLAGS_FILE: &str = "compile_flags.txt";

const COMPILER_KEY: &str = "compiler";
const DEBUGGER_KEY: &str = "debugger";
const FLAGS_KEY: &str = "flags";
const DEBUG_FLAGS_KEY: &str = "debug-flags";
const RELEASE_FLAGS_KEY: &str = "release-flags";

const DEFAULT_COMPILER: &str = "clang++";
const DEFAULT_DEBUGGER: &str = "lldb";
const DEFAULT_FLAGS: &str = "-Wall -Wextra -pedantic";
const DEFAULT_DEBUG_FLAGS: &str = "-g";
const DEFAULT_RELEASE_FLAGS: &str = "-O2 -s";

const SRC_DIR: &str = "src";
const INCLUDE_DIR: &str = "include";
const BUILD_DIR: &str = "build";

const DEBUG_BUILD_SUBDIR: &str = "debug";
const RELEASE_BUILD_SUBDIR: &str = "release";

#[cfg(target_os = "linux")]
static EXE_EXTENSION: &str = "";

#[cfg(target_os = "macos")]
static EXE_EXTENSION: &str = "";

#[cfg(target_os = "windows")]
static EXE_EXTENSION: &str = ".exe";

const HELLO_WORLD: &str = r#"#include <iostream>

int main() {
    std::cout << "Hello World!" << std::endl;
    return 0;
}
"#;

const SEPARATOR: char = std::path::MAIN_SEPARATOR;

struct Config {
    compiler: String,
    debugger: String,

    flags: String,
    debug_flags: String,
    release_flags: String,
}

fn read_string_key(toml: &Map<String, Value>, key_name: &str) -> Result<Option<String>, String> {
    if let Some(value) = toml.get(key_name) {
        if let Some(slice) = value.as_str() {
            Ok(Some(slice.to_string()))
        } else {
            Err(format!("{key_name} value must be a string"))
        }
    } else {
        Ok(None)
    }
}

fn read_configuration(config_path: &str) -> Result<Config, String> {
    match std::fs::read_to_string(format!("{config_path}{SEPARATOR}{CONFIG_FILE}")) {
        Ok(toml_str) => match toml_str.parse::<Table>() {
            Ok(toml) => {
                let compiler =
                    read_string_key(&toml, COMPILER_KEY)?.unwrap_or(DEFAULT_COMPILER.to_owned());
                let debugger =
                    read_string_key(&toml, DEBUGGER_KEY)?.unwrap_or(DEFAULT_DEBUGGER.to_owned());
                let flags = read_string_key(&toml, FLAGS_KEY)?.unwrap_or(DEFAULT_FLAGS.to_owned());
                let debug_flags = read_string_key(&toml, DEBUG_FLAGS_KEY)?
                    .unwrap_or(DEFAULT_DEBUG_FLAGS.to_owned());
                let release_flags = read_string_key(&toml, RELEASE_FLAGS_KEY)?
                    .unwrap_or(DEFAULT_RELEASE_FLAGS.to_owned());

                Ok(Config {
                    compiler,
                    debugger,
                    flags,
                    debug_flags,
                    release_flags,
                })
            }

            Err(_toml_parse_error) => Err(format!(
                "Can't parse {CONFIG_FILE} file ! Does it contain valid toml ?"
            )),
        },
        Err(_toml_read_error) => Err(format!(
            "Can't read {CONFIG_FILE} file ! Are you in a project folder ?"
        )),
    }
}

fn build(compiler: &str, flags: &str, build_subdir: &str) {
    let walker = WalkDir::new(SRC_DIR);

    let mut source_files = String::new();

    for entry in walker {
        match entry {
            Ok(file) => {
                if file.file_type().is_file() {
                    let file_name = file.file_name().to_string_lossy();

                    if file_name.ends_with(".c") || file_name.ends_with(".cpp") {
                        source_files.push_str(&file.path().to_string_lossy());
                        source_files.push(' ');
                    }
                }
            }

            Err(error) => {
                eprintln!("Error can't read entry : {error}");
                return;
            }
        }
    }

    if !std::path::Path::new(BUILD_DIR).is_dir() {
        if let Err(error) = std::fs::create_dir(BUILD_DIR) {
            eprintln!("Can't create {BUILD_DIR} directory : {error}");
            return;
        }
    }

    let subdir = format!("{BUILD_DIR}{SEPARATOR}{build_subdir}");
    if !std::path::Path::new(&subdir).is_dir() {
        if let Err(error) = std::fs::create_dir(&subdir) {
            eprintln!("Can't create {subdir} directory : {error}");
            return;
        }
    }

    let mut compile_command = Command::new(compiler);

    compile_command.args(flags.split_whitespace());
    compile_command.arg(format!("-o{subdir}{SEPARATOR}app{EXE_EXTENSION}"));
    compile_command.args(source_files.split_whitespace());

    let compile_result = compile_command.status();

    match compile_result {
        Ok(exit_status) => {
            if exit_status.success() {
                println!("App built successfully");
            } else {
                println!("Compilation error !");
            }
        }

        Err(error) => {
            eprintln!("Compiler error : {error}");
        }
    }
}

fn build_command() {
    match read_configuration(".") {
        Ok(config) => {
            build(
                &config.compiler,
                &format!("{} {} -I{}", config.flags, config.debug_flags, INCLUDE_DIR),
                DEBUG_BUILD_SUBDIR,
            );
        }
        Err(err_msg) => {
            eprintln!("{err_msg}");
        }
    }
}

fn release_build_command() {
    match read_configuration(".") {
        Ok(config) => {
            build(
                &config.compiler,
                &format!(
                    "{} {} -I{}",
                    config.flags, config.release_flags, INCLUDE_DIR
                ),
                RELEASE_BUILD_SUBDIR,
            );
        }
        Err(err_msg) => {
            eprintln!("{err_msg}");
        }
    }
}

fn run_command() {
    build_command();

    match read_configuration(".") {
        Ok(config) => {
            let mut run_command = Command::new(config.debugger);
            run_command.arg("--source-quietly");
            run_command.arg("-o");
            run_command.arg("run");
            run_command.arg("-o");
            run_command.arg("exit");
            run_command.arg(format!(
                "{BUILD_DIR}{SEPARATOR}{DEBUG_BUILD_SUBDIR}{SEPARATOR}app{EXE_EXTENSION}"
            ));

            if let Err(error) = run_command.status() {
                println!("Can't run your app : {error}");
            }
        }
        Err(err_msg) => {
            eprintln!("{err_msg}");
        }
    }
}

fn release_run_command() {
    release_build_command();

    let mut run_command = Command::new(format!(
        "{BUILD_DIR}{SEPARATOR}{RELEASE_BUILD_SUBDIR}{SEPARATOR}app{EXE_EXTENSION}"
    ));

    if let Err(error) = run_command.status() {
        println!("Can't run your app : {error}");
    }
}

fn debug_command() {
    build_command();

    match read_configuration(".") {
        Ok(config) => {
            let mut run_command = Command::new(config.debugger);
            run_command.arg(format!(
                "{BUILD_DIR}{SEPARATOR}{DEBUG_BUILD_SUBDIR}{SEPARATOR}app{EXE_EXTENSION}"
            ));

            if let Err(error) = run_command.status() {
                println!("Can't run your app : {error}");
            }
        }
        Err(err_msg) => {
            eprintln!("{err_msg}");
        }
    }
}

fn init_command() {
    if std::path::Path::new(CONFIG_FILE).is_file() {
        eprintln!("Can't init an already existing embargo project");
        return;
    }

    if !std::path::Path::new(SRC_DIR).is_dir() {
        match std::fs::create_dir(SRC_DIR) {
            Ok(()) => {
                if let Err(error) =
                    std::fs::write(format!("{SRC_DIR}{SEPARATOR}main.cpp"), HELLO_WORLD)
                {
                    eprintln!("Can't create default main.cpp file : {error}");
                }
            }

            Err(error) => {
                eprintln!("Can't create {SRC_DIR} dir : {error}");
            }
        }
    }

    if !std::path::Path::new(INCLUDE_DIR).is_dir() {
        if let Err(error) = std::fs::create_dir(INCLUDE_DIR) {
            eprintln!("Can't create {INCLUDE_DIR} dir : {error}");
        }
    }

    if !std::path::Path::new(CONFIG_FILE).is_file() {
        if let Err(error) = std::fs::write(CONFIG_FILE, "") {
            eprintln!("Can't create {CONFIG_FILE} : {error}");
        } else {
            clangd_config_command();
        }
    }
}

fn config_command() {
    match read_configuration(".") {
        Ok(config) => {
            println!("Embargo is configured as follow: ");
            println!("    Compiler          {}", config.compiler);
            println!("    Debugger          {}", config.debugger);
            println!("    Flags             {}", config.flags);
            println!("    Debug flags       {}", config.debug_flags);
            println!("    Release flags     {}", config.release_flags);
        }
        Err(err_msg) => {
            eprintln!("{err_msg}");
        }
    }
}

fn clangd_config_command() {
    match read_configuration(".") {
        Ok(config) => {
            let mut compile_flags = String::new();

            compile_flags.push_str("-Iinclude\n");
            compile_flags.push_str("-Isrc\n");

            for flag in config.flags.split_whitespace() {
                compile_flags.push_str(flag);
                compile_flags.push('\n');
            }

            if let Err(error) = std::fs::write(COMPILE_FLAGS_FILE, compile_flags) {
                eprintln!("Can't write {COMPILE_FLAGS_FILE} : {error}");
            }
        }
        Err(err_msg) => {
            eprintln!("{err_msg}");
        }
    }
}

fn clean_command() {
    if std::path::Path::new(BUILD_DIR).is_dir() {
        if let Err(error) = std::fs::remove_dir_all(BUILD_DIR) {
            eprintln!("Can't remove {BUILD_DIR} directory : {error}");
        }
    }
}

fn help_command() {
    println!("Usage: embargo [COMMAND]");
    println!("Available commands :");
    println!("    build            Build the app in debug mode");
    println!("    release-build    Build the app in release mode");
    println!("    run              Build the app in debug mode and run it");
    println!("    release-run      Build the app in release mode and run it");
    println!("    debug            Build the app in debug mode and open it with the debugger");
    println!("    init             Create default files to start working on your project");
    println!("    config           Show embargo configuration as defined in Embargo.toml file");
    println!("    clangd-config    Generate compile_flags.txt based on your configuration for clangd settings");
    println!("    clean            Remove the build directory");
    println!("    help             Show this help message");
}

fn main() {
    if let Some(first_arg) = std::env::args().nth(1) {
        match first_arg.as_str() {
            "build" => {
                build_command();
            }
            "release-build" => {
                release_build_command();
            }
            "run" => {
                run_command();
            }
            "release-run" => {
                release_run_command();
            }
            "debug" => {
                debug_command();
            }
            "init" => {
                init_command();
            }
            "config" => {
                config_command();
            }
            "clangd-config" => {
                clangd_config_command();
            }
            "clean" => {
                clean_command();
            }
            "help" => {
                help_command();
            }
            _ => {
                eprintln!("Unknown command, try `embargo help` for more informations");
            }
        }
    } else {
        eprintln!("Embargo takes a command as parameter, try `embargo help` for more informations");
    }
}
