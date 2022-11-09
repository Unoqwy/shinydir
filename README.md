# shinydir - Enforce clean directories!

Stop letting yourself throw junk files everywhere, set rules and let a tool yell at you.

![Demo GIF](./demo/demo.gif)

shinydir is a CLI tool that checks if directories are filled with allowed files or sub-directories according to set rules and report any outliar. It can also automatically move misplaced files according to configured rules.

## Installation

### From crates.io

The recommended way to install shinydir is from crates.io by using `cargo install`.  

Make sure you have a recent version of the rust toolchain installed.  
To build the latest published version and install it, run `crate install shinydir`.

### From GitHub releases

If you do not have the Rust toolchain installed or building on your machine is inconvenient, you can download compiled binaries directly from the latest release on Github.

### From source

Make sure you have a recent version of the rust toolchain installed.

1. Clone this repository: `git clone https://github.com/Unoqwy/shinydir`
2. Go into the cloned directory: `cd shinydir`
3. Build and install: `cargo install --path .`

## Usage

You may set the configuration file path with the environment variable `SHINYDIR_CONFIG_FILE` (using an absolute path is recommended). Otherwise, the default path depends on your operating system.
* Linux: `$XDG_CONFIG_HOME/shinydir/shinydir.toml` (`$XDG_CONFIG_HOME` defaults to `$HOME/.config` if unset)
* MacOS: `$HOME/Library/Application Support/Shiny-Dir/shinydir.toml`
* Windows: `C:\Users\User\AppData\Roaming\Shiny Dir\config\shinydir.toml`
The default config is copied on first run if the config file doesn't exist. You can check out the defaults [here](./shinydir.toml).

The power of shinydir comes from your own configuration, make sure to tailor the config file to your liking before proceeding.

`shd [-c <custom-config>] check [--list]` - Checks configured directories and reports misplaced files. Use `--list` to get a raw list of absolute filenames without formatting, useful for piping.

`shd [-c <custom-config>] auto-move [--dry] [--list]` - Moves misplaced files according to set rules. Use `--dry` to see what would be the outcome without actually moving files. Use `--list` to get a raw list of "move actions", each line will have the old file path and new file path separated by a space. You may call external scripts to get a file's destination in the config (see advanced rules), so you can get creative.

## Configuration

### General Settings

```toml
[settings]
color = true # Use colors in terminal output
use-unicode = true # Use fancy unicode chars in terminal output
hide-ok-directories = true # Hide directories (and auto-move rules) from output
                           # when there are no misplaced files/nothing to move
```

### Directory Rules

Set the rules you want, so that the `check` command can yell at you accordingly.


```toml
[dir."$HOME/Shared"] # Create a rule for the 'Shared' directory is user home
allowed-dirs = [
    # Allow only two sub-directories: 'Movies', and 'FileTransfer'
    # Any other directory will be reported as misplaced
    { name = "Movies" },
    { name = "FileTransfer" },
]
allowed-files = [
    # Allow files prefixed with `backup-`
    # Any other file will be reported as misplaced
    { pattern = "^backup-" },
]

[dir."$HOME/Shared/Movies"]
recursive = true # Apply rules recursively on child directories
recursive-ignore-children = [
    # Recursive rules will not apply in the directory '.thumbnails'
    { name = ".thumbnails" },
]
# Omitting 'allow-dirs' means any directory is allowed
allow-files = [
    # Allow files with extension 'mp4' or 'mov'
    # Any other file will be reported as misplaced
    { ext = "mp4" },
    { ext = "mov" },
]

```

### Auto-Move General Settings

```toml
[automove]
force-dry-run = true # This is a security feature for new users.
                     # It prevents auto-move from actually working!
                     # Make sure to turn this off (set to false) when you are ready to use auto-move
```

### Auto-Move Rules

Set the rules you want, so that the `auto-move` command can sort the mess for you.

```toml
[[automove.rules]]
name = "Home Movies" # Give a custom name to the rule. If unset, it will fallback to the absolute path of `parent`
parent = "$HOME/Shared" # Handle files under in the 'Shared' directory
match = [
    # Look for files with extension 'mp4' or 'mov'
    { ext = "mp4" },
    { ext = "mov" },
]
to = "$HOME/Shared/Movies" # These files belong to our 'Movies' sub-directory, move them there
```

You can deal with more complex scenarios using scripts. In the following examples, screenshots are automatically sorted in monthly directories. You can check out the associated script [here](./examples/dated-monthly.sh).

```toml
[[automove.rules]]
name = "Screenshots"
parent = "$HOME/Screenshots"
match = [
    { ext = "png" },
    { ext = "jpg" },
]
to = "$HOME/Screenshots" # Stay within the same directory with this setting,
                         # the script below returns a relative filename that includes a directory
to-script = "examples/dated-monthly.sh" # This script will be called for each matching file,
                                        # to get a new filename to move the file to.
                                        # In this example, a file created in November 2022 will be moved to
                                        # `$HOME/Screenshots/Nov-2022/<original filename>`
# Note: `to` is still required when `to-script` is set! It will be the base path of any relative filename the script returns.
#       However, the script is free to return absolute paths, in which case `to` will be ignored
```
