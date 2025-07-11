# Cliffcrown
A simple greeter for greetd.

## Usage
This command should be run inside a display manager. I recommend running Sway with a custom configuration which launches this program into a new fullscreen window on startup.

## Configuration
Options are given as "CLI args; config file option".

`-u`, `--user`; `restricted_user`: skip asking what user to use and attempt to login with this one instead.

`-b`, `--bg`; `background`: load image from given path and use that as the background

`-c`, `--config`; none: access config file from given path instead of the default `/etc/greetd/cliffcrown.toml`

after `--`; `command`: list of strings which will be used as the command to launch on a successful authorisation
