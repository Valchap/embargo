# Embargo
Embargo is a simple and fast opinionated build system for c/c++

## Install or update
Install [rust](https://www.rust-lang.org/learn/get-started)

Then install embargo with rust package manager :
```sh
cargo install embargo
```

Embargo uses the llvm toolchain by default (clang, lldb, clang-tidy)

Debian dependencies :
```sh
apt install clang lldb clang-tidy
```

## Usage
### Create a new project
First create a project folder, then type the following command inside it:
```sh
embargo init
```
This will create a default "Hello World"

### Build your app
Debug build :
```sh
embargo build
```

Release build :
```sh
embargo release-build
```

Builds can be found in the `build/debug` or `build/release` folder

### Run your app
Debug run :
```sh
embargo run
```

Release run :
```sh
embargo release-run
```
Embargo will build your app before running it so that you always run the latest version of your app

`embargo run` runs your app inside a debugger so that you can easily find where a crash happened in your code.

### Debug your app
```sh
embargo debug
```
This will start the debugger with your app attached to it

### Lint your app
```sh
embargo lint
```
This will use `clang-tidy` to find common mistakes in your code

### Generate clangd configuration
```sh
embargo clangd-config
```
This will create the `compile_flags.txt` that can the be used by the clangd language server

### Clean build folder
```sh
embargo clean
```
This will remove the build folder

### Show configuration
```sh
embargo show-config
```
This will show Embargo configuration for the current project

## Configuration
Embargo project configuration is read from the Embargo.toml file at the root of your project

Here is an example configuration with Embargo default settings

```toml
compiler = "clang"
debugger = "lldb"
linter = "clang-tidy"
flags = ["-Wall", "-Wextra", "-pedantic"]
debug-flags = ["-g"]
release-flags = ["-O2"]
linker-flags = []
linter-checks = ["clang-analyzer-*"]
```

If a key is missing in the configuration Embargo will use these as default settings

## Alternatives
If you don't want to install the rust toolchain, but still want similar functionality, you may have a look at [PyBargo](https://github.com/charyan/PyBargo)
