use clap::{Arg, ArgAction, Command};
use crossterm::{
  execute,
  style::{Attribute, Color, Colors, Print, ResetColor, SetAttribute, SetColors},
};
use regex::Regex;
use std::{io::stdout, process::ExitCode};

const VERSION: &str = env!("CARGO_PKG_VERSION");

mod downloader;
mod web;
use downloader::download_folder;
mod google_drive;
use google_drive::process_folder_id;

#[tokio::main]
async fn main() -> ExitCode {
  let matches = Command::new("gdown")
    .about("Download Google-Drive shares recursively through the command line.")
    .version(VERSION)
    .author("DepriSheep")
    .arg_required_else_help(true)
    .arg(
      Arg::new("id")
        .help("An alpha-numeric string with 33 total characters.")
        .required(true)
        .action(ArgAction::Set)
        .num_args(1),
    )
    .arg(
      Arg::new("force")
        .short('f')
        .long("force")
        .help("Overwrite files when necessary.")
        .required(false)
        .action(ArgAction::SetTrue),
    )
    .arg(
      Arg::new("recursive")
        .short('R')
        .long("non-recursively")
        .help("Don't download folders recursively.")
        .required(false)
        .action(ArgAction::SetFalse),
    )
    .arg(
      Arg::new("md5")
        .short('c')
        .long("check")
        .help("Check integrity of files (MD5).")
        .required(false)
        .action(ArgAction::SetTrue),
    )
    .arg(
      Arg::new("verbose")
        .short('v')
        .long("verbose")
        .help("Print all warning messages.")
        .required(false)
        .action(ArgAction::SetTrue),
    )
    .arg(
      Arg::new("no-download")
        .long("no-download")
        .help("Don't download anything, just announce changes.")
        .required(false)
        .action(ArgAction::SetTrue),
    )
    .arg(
      Arg::new("output-folder")
        .short('o')
        .long("output-folder")
        .help("How to name the root folder (by default the folder-id).")
        .required(false)
        .action(ArgAction::Set),
    )
    .arg(
      Arg::new("file-id")
        .long("file-id")
        .help("If you have a file-id instead of a folder-id.")
        .required(false)
        .action(ArgAction::SetTrue),
    )
    .get_matches();

  match matches.args_present() {
    true => {
      let id = matches.get_one::<String>("id").unwrap();
      let force = matches.get_flag("force");
      let recursive = matches.get_flag("recursive");
      let md5 = matches.get_flag("md5");
      let verbose = matches.get_flag("verbose");
      let no_download = matches.get_flag("no-download");

      let output_folder =
        if let Some(output_folder_arg) = matches.get_one::<String>("output-folder") {
          output_folder_arg
        } else {
          id
        };

      let basic_reg = Regex::new("[[a-zA-Z0-9]-_]{33}").unwrap();
      if !basic_reg.is_match(id) || id.contains("http") {
        print_message(
          MessageType::Error,
          "Invalid ID format. Please ensure you're using the correct format: [[a-zA-Z0-9]-_]{33}.",
        );
        return ExitCode::FAILURE;
      }

      if matches.get_flag("file-id") {
        print_message(
          MessageType::Error,
          format!("Just do: \"\
wget --content-disposition \'https://drive.usercontent.google.com/download?id={}&export=download&confirm=t\'\"", id).as_str());
        return ExitCode::SUCCESS; // hehe
      }

      let google_files = process_folder_id(id, output_folder, verbose).await.unwrap();

      download_folder(google_files, force, recursive, md5, verbose, no_download).await;

      ExitCode::SUCCESS
    },
    _ => ExitCode::FAILURE,
  }
}

enum MessageType {
  Warning,
  Info,
  Error,
}

fn print_message(message_type: MessageType, message: &str) {
  let (prefix, foreground) = match message_type {
    MessageType::Warning => ("Warning: ", Color::Yellow),
    MessageType::Error => ("Error: ", Color::Red),
    MessageType::Info => ("Info: ", Color::Blue),
  };
  execute!(
    stdout(),
    SetAttribute(Attribute::Bold),
    SetColors(Colors::new(foreground, Color::Reset)),
    Print(prefix.to_string()),
    ResetColor,
    SetAttribute(Attribute::Bold),
    Print(message.to_string()),
    ResetColor,
  )
  .unwrap();
  println!();
}
