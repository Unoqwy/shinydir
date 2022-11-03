# shinydir - Enforce clean directories!

Stop letting yourself throw junk files everywhere, set rules and let a tool yell at you.

shinydir is a CLI tool that checks if directories are filled with allowed files or sub-directories according to set rules and report any outliar. It can also automatically move misplaced files according to configured rules.

## Configuration

The configuration file is located at `$XDG_CONFIG_HOME/shinydir/shinydir.toml` (i.e. `/home/user/.config/shinydir/shinydir.toml`).  
The defaults are copied if the file doesn't exist. You can check out the default configuration [here](./shinydir.toml).

