# Cliffcrown
A simple greeter for greetd.

## Configuration
Options are given as "CLI args; config file option".

`-u`, `--user`; `restricted_user`: skip asking what user to use and attempt to login with this one instead.

`-b`, `--bg`; `background`: load image from given path and use that as the background

`-c`, `--config`; none: access config file from given path instead of the default `/etc/greetd/cliffcrown.toml`

after `--`; `command`: list of strings which will be used as the command to launch on a successful authorisation
