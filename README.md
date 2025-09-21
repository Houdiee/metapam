# Metapam - Declarative meta-package manager
A tool to declaratively manager globally installed packages for supported package managers. Made for those who want a similar experience to the nix package manager without its restrictiveness and want things to "just work" out of the box.
## Getting Started
The term `provider` can be used interchangeably with `package manager` in this README, but this is not the case for the cli.

* To get started, run `metapam activate`, which will create a configuration for each package manager in `XDG_CONFIG_DIR/metapam/`.

* To create a configuration only for a specific provider run `metapam provider <PROVIDER_NAME> activate`.

* To install a package and add it to the config, run `metapam provider <PROVIDER_NAME> declare <PACKAGES>`.
* To remove a package and remove it from the config, run `metapam provider <PROVIDER_NAME> remove <PACKAGES>`.

### Example Usage
configure your global packages for a given package manager, like so in `XDG_CONFIG_DIR/metapam/<PROVIDER_NAME>`:
```
neovim
vim
# newline comments are allowed
firefox
opam
// double slashes are also welcome
cargo
go

--- doing this is not allowed ---
cargo # a comment next to a package
```

### Tidy command
If you have installed or removed packages from your system, that don't match the declared configuration, run `metapam provider <PROVIDER_NAME> tidy` which will do 2 things:
1. Install packages that are declared but not installed
2. Remove packages that aren't declared but exist on the filesystem

## Installation/Build from source
1. `git clone https://github.com/Houdiee/metapam.git && cd metapam`
2. `cargo build --release`
3. `mv target/release/metapam ~/.local/share/bin` or move it anywhere else that is a valid PATH
   
now you can run `metapam`
   
## Supported package managers

| Provider Name| Supported |
|--------------|-----------|
|apt|✅|
|brew|✅
|cargo|✅
|dotnet|✅
|go|✅
|npm|✅
|pnpm|✅
|pacman|✅
|paru|✅
|yay|✅

IF YOU WOULD LIKE A PACKAGE MANAGER TO BE ADDED, OPEN AN ISSUE
