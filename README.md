# metapam
CLI tool to declare all packages in your system by having a dedicated file per-package manager. Each file is simply a list of packages which describe which packages should be globally installed/removed.

This setup allows a degree of reproducibility across different systems, since a different existing machine will install all declared packages whilst removing all redundant (un-declared) packages to match the user's configured ideal system state.

### Example
Assume we want to descibe the ideal state for the package manager "homebrew". In `~/.config/metapam/brew`
```txt
# ~/.config/metapam/brew
vim
git
wget
curl
```

Running `metapam provider brew tidy` will manage hombrew by executing the following commands:
- `brew install <package>` for each declared package
- `brew uninstall <package>` for all packages which are remaining on the system, but not declared

This configuration is of course not limited to only homebrew. See [supported package managers](#-supported-package-managers)

### Getting Started
To auto-detect all currently installed packages and create a file for each package manager, simply run `metapam activate`. This will populate `~/.config/metapam/` with files describing the current state of each package manager.

### Commands
| Command | Action |
| - | - |
| `list available` | lists supported package managers by metapam |
| `list active` | lists currently active package managers managed by metapam |
| `activate` | populates metapam directory to declaratively reflect the state of each package manager |
| `tidy` | installs/removes all packages across every package manager to match the declared states |
| `diff` | displays additional or missing packages in the current system compared to the declared states |
| `provider <package manager> list` | lists installed packages for a specific package manager |
| `provider <package manager> activate` | activates only for a specific package manager |
| `provider <package manager> tidy` | runs `tidy` only for a specific package manager |
| `provider <package manager> diff` | runs `diff` only for a specific package manager |
| `provider <package manager> add` | adds a package to the package manager file without installing |
| `provider <package manager> declare` | adds a package to the package manager file and installs it |
| `provider <package manager> add-undeclared` | adds all undeclared packages to the package manager file |
| `provider <package manager> remove` | removes a package from the package manager file and uninstalls it |

## Installation/Build
1. `git clone https://github.com/Houdiee/metapam.git && cd metapam`
2. `cargo build --release`
3. Place the built binary into your system $PATH
   
## Supported package managers
| Provider Name | Supported |
|-|-|
|apt|✅|
|brew|✅
|cargo|✅
|dotnet|✅
|npm|✅
|pnpm|✅
|pacman|✅
|paru|✅
|yay|✅
|fisher|✅|
