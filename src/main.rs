#![warn(clippy::pedantic)]

mod parallel_runner;

use std::{fs::create_dir_all, path::Path, process::Command};

use parallel_runner::parallel_run;
use toml::{map::Map, Table, Value};
use walkdir::WalkDir;

const CONFIG_FILE: &str = "Embargo.toml";
const COMPILE_FLAGS_FILE: &str = "compile_flags.txt";

const COMPILER_KEY: &str = "compiler";
const DEBUGGER_KEY: &str = "debugger";
const LINTER_KEY: &str = "linter";

const FLAGS_KEY: &str = "flags";
const DEBUG_FLAGS_KEY: &str = "debug-flags";
const RELEASE_FLAGS_KEY: &str = "release-flags";

const LINTER_CHECKS_KEY: &str = "linter-checks";

const DEFAULT_COMPILER: &str = "clang++";
const DEFAULT_DEBUGGER: &str = "lldb";
const DEFAULT_LINTER: &str = "clang-tidy";

const DEFAULT_FLAGS: &[&str] = &["-Wall", "-Wextra", "-pedantic"];
const DEFAULT_DEBUG_FLAGS: &[&str] = &["-g"];
const DEFAULT_RELEASE_FLAGS: &[&str] = &["-O2", "-s"];

const DEFAULT_LINTER_CHECKS: &str = "clang-analyzer-*";

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
    linter: String,

    flags: Vec<String>,
    debug_flags: Vec<String>,
    release_flags: Vec<String>,

    linter_checks: String,
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

fn read_string_list_key(
    toml: &Map<String, Value>,
    key_name: &str,
) -> Result<Option<Vec<String>>, String> {
    let mut values = Vec::new();

    if let Some(value) = toml.get(key_name) {
        if let Some(array) = value.as_array() {
            for v in array {
                if let Some(slice) = v.as_str() {
                    values.push(slice.to_owned());
                } else {
                    return Err(format!("{key_name} value must be an array of string"));
                }
            }

            Ok(Some(values))
        } else {
            Err(format!("{key_name} value must be an array of string"))
        }
    } else {
        Ok(None)
    }
}

fn to_owned_string_vec(in_list: &[&str]) -> Vec<String> {
    let mut out_list = Vec::new();

    for &s in in_list {
        out_list.push(s.to_string());
    }

    out_list
}

fn default_configuration() -> Config {
    Config {
        compiler: DEFAULT_COMPILER.to_owned(),
        debugger: DEFAULT_DEBUGGER.to_owned(),
        linter: DEFAULT_LINTER.to_owned(),
        flags: to_owned_string_vec(DEFAULT_FLAGS),
        debug_flags: to_owned_string_vec(DEFAULT_DEBUG_FLAGS),
        release_flags: to_owned_string_vec(DEFAULT_RELEASE_FLAGS),
        linter_checks: DEFAULT_LINTER_CHECKS.to_owned(),
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
                let linter =
                    read_string_key(&toml, LINTER_KEY)?.unwrap_or(DEFAULT_LINTER.to_owned());

                let flags = read_string_list_key(&toml, FLAGS_KEY)?
                    .unwrap_or(to_owned_string_vec(DEFAULT_FLAGS));
                let debug_flags = read_string_list_key(&toml, DEBUG_FLAGS_KEY)?
                    .unwrap_or(to_owned_string_vec(DEFAULT_DEBUG_FLAGS));
                let release_flags = read_string_list_key(&toml, RELEASE_FLAGS_KEY)?
                    .unwrap_or(to_owned_string_vec(DEFAULT_RELEASE_FLAGS));

                let linter_checks = read_string_key(&toml, LINTER_CHECKS_KEY)?
                    .unwrap_or(DEFAULT_LINTER_CHECKS.to_owned());

                Ok(Config {
                    compiler,
                    debugger,
                    linter,
                    flags,
                    debug_flags,
                    release_flags,
                    linter_checks,
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

fn find_file(dir: &str, extensions: &[&str]) -> Result<Vec<String>, String> {
    let mut files = Vec::new();

    let walker = WalkDir::new(dir);

    for entry in walker {
        match entry {
            Ok(file) => {
                if file.file_type().is_file() {
                    let file_name = file.file_name().to_string_lossy();

                    for ext in extensions {
                        if file_name.ends_with(ext) {
                            files.push((*file.path().to_string_lossy()).to_owned());
                            break;
                        }
                    }
                }
            }

            Err(error) => {
                return Err(format!("Error can't read entry : {error}"));
            }
        }
    }

    Ok(files)
}

fn find_srcs() -> Result<Vec<String>, String> {
    find_file(SRC_DIR, &[".cpp", ".c"])
}

fn find_code() -> Result<Vec<String>, String> {
    find_file(SRC_DIR, &[".hpp", ".h", ".cpp", ".c"])
}

fn find_objects(build_subdir: &str) -> Result<Vec<String>, String> {
    find_file(&format!("{BUILD_DIR}{SEPARATOR}{build_subdir}"), &[".o"])
}

fn compile_object(options: (String, Vec<String>, String, String)) -> Result<bool, String> {
    let compiler = options.0;
    let flags = options.1;
    let input = options.2;
    let output = options.3;

    let mut compile_command = Command::new(compiler);

    compile_command.args(flags);
    compile_command.arg("-c");
    compile_command.arg(format!("-o{output}"));
    compile_command.arg(input);

    let path = Path::new(&output);
    if let Some(parent_dir) = path.parent() {
        if let Err(error) = create_dir_all(parent_dir) {
            return Err(format!("Can't create build folder : {error}"));
        }
    }

    let compile_result = compile_command.status();

    match compile_result {
        Ok(compile_output) => {
            if compile_output.success() {
                Ok(true)
            } else {
                Ok(false)
            }
        }

        Err(error) => Err(format!("Can't run compiler : {error}")),
    }
}

fn compile_all_objects(compiler: &str, flags: &[&str], build_subdir: &str) -> Result<bool, String> {
    let source_files = match find_srcs() {
        Ok(srcs) => srcs,
        Err(error) => {
            return Err(error);
        }
    };

    let mut compile_parameters = Vec::new();

    for source_file in source_files {
        let compiler_s = compiler.to_owned();
        let mut flags_s = Vec::new();
        for &flag in flags {
            flags_s.push(flag.to_owned());
        }
        let input_s = source_file;
        let output_s = format!(
            "{BUILD_DIR}{SEPARATOR}{build_subdir}{SEPARATOR}{}",
            Path::new(&input_s).with_extension("o").to_string_lossy()
        );

        compile_parameters.push((compiler_s, flags_s, input_s, output_s));
    }

    let results = parallel_run(compile_parameters, compile_object);

    for result in results {
        match result {
            Ok(build_successful) => {
                if !build_successful {
                    return Ok(false);
                }
            }
            Err(compiler_error) => return Err(compiler_error),
        }
    }

    Ok(true)
}

fn link_program(compiler: &str, flags: &[&str], build_subdir: &str) -> Result<bool, String> {
    let obj_files = match find_objects(build_subdir) {
        Ok(objects) => objects,
        Err(error) => {
            return Err(error);
        }
    };

    let subdir = format!("{BUILD_DIR}{SEPARATOR}{build_subdir}");
    if let Err(error) = create_dir_all(&subdir) {
        return Err(format!("Can't create {subdir} directory : {error}"));
    }

    let mut link_command = Command::new(compiler);

    link_command.args(flags);
    link_command.arg(format!("-o{subdir}{SEPARATOR}app{EXE_EXTENSION}"));
    link_command.args(obj_files);

    let link_result = link_command.status();

    match link_result {
        Ok(exit_status) => {
            if exit_status.success() {
                Ok(true)
            } else {
                Ok(false)
            }
        }

        Err(error) => Err(format!("Compiler error : {error}")),
    }
}

fn build(config: &Config, release: bool) -> Result<bool, String> {
    let mut flags = Vec::<&str>::new();

    for f in &config.flags {
        flags.push(f);
    }

    if release {
        for f in &config.release_flags {
            flags.push(f);
        }
    } else {
        for f in &config.debug_flags {
            flags.push(f);
        }
    }

    let include_flag = format!("-I{INCLUDE_DIR}");
    flags.push(&include_flag);

    let build_subdir = if release {
        RELEASE_BUILD_SUBDIR
    } else {
        DEBUG_BUILD_SUBDIR
    };

    if compile_all_objects(&config.compiler, &flags, build_subdir)? {
        link_program(&config.compiler, &flags, build_subdir)
    } else {
        Ok(false)
    }
}

fn lint(linter: &str, checks: &str, compile_flags: &[&str]) {
    let code_files = match find_code() {
        Ok(files) => files,
        Err(error) => {
            eprintln!("{error}");
            return;
        }
    };

    let mut lint_command = Command::new(linter);

    lint_command.args(code_files);
    lint_command.arg(format!("-checks={checks}"));
    lint_command.arg("--");
    lint_command.args(compile_flags);

    let lint_result = lint_command.status();

    match lint_result {
        Ok(exit_status) => {
            if exit_status.success() {
                println!("Linting successfull");
            } else {
                println!("Linter error !");
            }
        }

        Err(error) => {
            eprintln!("Linter error : {error}");
        }
    }
}

fn build_command(config: &Config) {
    match build(config, false) {
        Ok(successful) => {
            if successful {
                println!("Build successful");
            } else {
                println!("Build failed");
            }
        }
        Err(err_msg) => {
            eprintln!("Internal error : {err_msg}");
        }
    }
}

fn release_build_command(config: &Config) {
    match build(config, true) {
        Ok(successful) => {
            if successful {
                println!("Build successful");
            } else {
                println!("Build failed");
            }
        }
        Err(err_msg) => {
            eprintln!("Internal error : {err_msg}");
        }
    }
}

fn run_command(config: &Config) {
    build_command(config);

    let mut run_command = Command::new(&config.debugger);
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

fn release_run_command(config: &Config) {
    release_build_command(config);

    let mut run_command = Command::new(format!(
        "{BUILD_DIR}{SEPARATOR}{RELEASE_BUILD_SUBDIR}{SEPARATOR}app{EXE_EXTENSION}"
    ));

    if let Err(error) = run_command.status() {
        println!("Can't run your app : {error}");
    }
}

fn debug_command(config: &Config) {
    build_command(config);

    let mut run_command = Command::new(&config.debugger);
    run_command.arg(format!(
        "{BUILD_DIR}{SEPARATOR}{DEBUG_BUILD_SUBDIR}{SEPARATOR}app{EXE_EXTENSION}"
    ));

    if let Err(error) = run_command.status() {
        println!("Can't run your app : {error}");
    }
}

fn lint_command(config: &Config) {
    let mut flags = Vec::<&str>::new();

    for f in &config.flags {
        flags.push(f);
    }

    let f = format!("-I{INCLUDE_DIR}");
    flags.push(&f);

    lint(&config.linter, &config.linter_checks, &flags);
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
            clangd_config_command(&default_configuration());
        }
    }
}

fn show_config_command(config: &Config) {
    println!("Embargo is configured as follow: ");
    println!("    Compiler          {}", config.compiler);
    println!("    Debugger          {}", config.debugger);
    println!("    Linter            {}", config.linter);
    println!("    Flags             {:?}", config.flags);
    println!("    Debug flags       {:?}", config.debug_flags);
    println!("    Release flags     {:?}", config.release_flags);
}

fn clangd_config_command(config: &Config) {
    let mut compile_flags = String::new();

    compile_flags.push_str("-Iinclude\n");
    compile_flags.push_str("-Isrc\n");

    for flag in &config.flags {
        compile_flags.push_str(flag);
        compile_flags.push('\n');
    }

    if let Err(error) = std::fs::write(COMPILE_FLAGS_FILE, compile_flags) {
        eprintln!("Can't write {COMPILE_FLAGS_FILE} : {error}");
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
    println!("    lint             Run the linter on your project to find common mistakes");
    println!("    init             Create default files to start working on your project");
    println!("    show-config      Show embargo configuration as defined in Embargo.toml file");
    println!("    clangd-config    Generate compile_flags.txt based on your configuration for clangd settings");
    println!("    clean            Remove the build directory");
    println!("    help             Show this help message");
}

fn main() {
    if let Some(first_arg) = std::env::args().nth(1) {
        match first_arg.as_str() {
            // Commands usable outside project directory
            "init" => init_command(),
            "help" => help_command(),

            _ => {
                match read_configuration(".") {
                    Ok(config) => {
                        match first_arg.as_str() {
                            // Commands for use inside a project
                            "build" => build_command(&config),
                            "release-build" => release_build_command(&config),
                            "run" => run_command(&config),
                            "release-run" => release_run_command(&config),
                            "debug" => debug_command(&config),
                            "lint" => lint_command(&config),
                            "show-config" => show_config_command(&config),
                            "clangd-config" => clangd_config_command(&config),
                            "clean" => clean_command(), // Doesn't need configuration, but for safety can only be used inside a project
                            _ => eprintln!(
                                "Unknown command, try `embargo help` for more informations"
                            ),
                        }
                    }
                    Err(err_msg) => {
                        eprintln!("{err_msg}");
                    }
                }
            }
        }
    } else {
        eprintln!("Embargo takes a command as parameter, try `embargo help` for more informations");
    }
}
