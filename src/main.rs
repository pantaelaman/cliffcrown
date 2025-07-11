#![feature(sync_unsafe_cell)]
use std::{fs::File, io::Read};

use clap::Parser;
use itertools::Itertools;
use serde::Deserialize;

mod client;
mod gui;
mod util;

const DEFAULT_CONFIG_PATH: &'static str = "/etc/greetd/cliffcrown.toml";
const DEFAULT_COMMAND: [&'static str; 1] = ["bash"];

#[derive(Deserialize, Default)]
struct StashedConfig {
  restricted_user: Option<String>,
  command: Option<Vec<String>>,
  #[serde(rename = "background")]
  bg_image: Option<String>,
}

struct Config {
  restricted_user: Option<String>,
  command: Vec<String>,
  bg_image: Option<String>,
}

#[derive(Parser, Debug)]
struct CLIArgs {
  #[arg(short = 'u', long = "user")]
  restricted_user: Option<String>,
  #[arg(short = 'b', long = "bg")]
  bg_image: Option<String>,
  #[arg(short = 'C', long = "config", default_value = DEFAULT_CONFIG_PATH)]
  config_path: String,
  #[arg()]
  command: Option<Vec<String>>,
}

#[tokio::main]
async fn main() {
  let args = CLIArgs::parse();
  let stashed_config: StashedConfig = File::open(args.config_path)
    .inspect_err(|e| println!("couldn't open file: {e}"))
    .ok()
    .and_then(|mut cfile| {
      let mut contents = String::new();
      cfile
        .read_to_string(&mut contents)
        .inspect_err(|e| println!("couldn't read file: {e}"))
        .ok()?;
      toml::de::from_str(&contents)
        .inspect_err(|e| println!("couldn't parse toml: {e}"))
        .ok()
    })
    .inspect(|_: &StashedConfig| println!("config exists before default"))
    .inspect(|c| println!("with background path {:?}", c.bg_image))
    .unwrap_or_default();

  let config = Config {
    restricted_user: args.restricted_user.or(stashed_config.restricted_user),
    bg_image: args.bg_image.or(stashed_config.bg_image),
    command: args
      .command
      .or(stashed_config.command)
      .unwrap_or_else(|| DEFAULT_COMMAND.into_iter().map_into().collect_vec()),
  };

  //eprintln!("{:?}", gui::GUI.run(config));

  let native_options = eframe::NativeOptions {
    window_builder: Some(Box::new(|v| v.with_maximized(true))),
    ..Default::default()
  };
  eframe::run_native(
    "CliffCrown",
    native_options,
    Box::new(|cc| Ok(Box::new(gui::GUI::new(cc, config)))),
  )
  .unwrap();
}
