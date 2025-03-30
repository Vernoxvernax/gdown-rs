use async_trait::async_trait;
use futures_util::StreamExt;
use indicatif::{HumanBytes, ProgressBar, ProgressStyle};
use std::{
  cmp::min,
  fs::{self, create_dir_all, File},
  io::{self, Write},
  path::Path,
  thread,
  time::{Duration, Instant},
};

use crate::{
  google_drive::{GoogleItem, GooglePage},
  print_message,
  web::api_get_file,
  MessageType,
};

#[async_trait]
pub trait Download {
  async fn download_content(
    &mut self,
    force: bool,
    md5: bool,
    verbose: bool,
    no_download: bool,
    total_files: &mut GooglePage,
  );
  async fn download_file(
    &mut self,
    force: bool,
    md5: bool,
    verbose: bool,
    no_download: bool,
    total_files: &mut GooglePage,
  );
  fn check_hash(&mut self, path: &Path, force: bool, no_download: bool, verbose: bool) -> bool;
  fn create_path(&mut self, verbose: bool);
}

#[async_trait]
impl Download for GoogleItem {
  async fn download_content(
    &mut self,
    force: bool,
    md5: bool,
    verbose: bool,
    no_download: bool,
    total_files: &mut GooglePage,
  ) {
    if self.mimeType == "application/vnd.google-apps.folder" {
      if let Some(children) = &mut self.children {
        for file in children.iter_mut() {
          file
            .download_content(force, md5, verbose, no_download, total_files)
            .await;
        }
      }
    } else {
      self
        .download_file(force, md5, verbose, no_download, total_files)
        .await;
    }
  }

  async fn download_file(
    &mut self,
    force: bool,
    md5: bool,
    verbose: bool,
    no_download: bool,
    total_files: &mut GooglePage,
  ) {
    if !no_download {
      self.create_path(verbose);
    }
    let path_str = format!("{}/{}", self.path.clone().unwrap(), self.title);
    let path: &Path = Path::new(&path_str);
    if !path.exists() || force {
      if no_download {
        total_files.items.push(self.clone());
        return;
      }
      print_message(
        MessageType::Info,
        format!("Started download for file: \"{}\"", path.display()).as_str(),
      );
      thread::sleep(Duration::from_millis(250));

      let pb = ProgressBar::new_spinner().with_style(
        ProgressStyle::with_template(
          "[{spinner:.blue}]-[{wide_bar:.blue/grey}] [{bytes}/{total_bytes}] {msg} ({eta})",
        )
        .unwrap()
        .tick_chars("-\\|/!")
        .progress_chars("=>-"),
      );
      pb.enable_steady_tick(Duration::from_millis(100));

      if let Ok(res) = api_get_file(self.id.clone()).await {
        let start_time = Instant::now();
        let mut last_check = start_time;
        let mut bytes_since_last_check = 0;
        let total_size = res.content_length().unwrap();

        let mut dest = File::create(path).unwrap();

        let mut progress: u64 = 0;
        let mut stream = res.bytes_stream();

        pb.set_length(total_size);

        while let Some(response_chunk) = stream.next().await {
          let file_chunk = response_chunk.unwrap();
          let chunk_size = file_chunk.len() as u64;
          dest.write_all(&file_chunk).unwrap();

          progress = min(progress + chunk_size, total_size);
          bytes_since_last_check += chunk_size;
          let now = Instant::now();
          let elapsed = now.duration_since(last_check);

          if elapsed >= Duration::from_secs(1) {
            let speed = bytes_since_last_check as f64 / elapsed.as_secs_f64();
            pb.set_message(format!("[{}/s]", HumanBytes(speed as u64)));

            last_check = now;
            bytes_since_last_check = 0;
          }

          pb.set_position(progress);
        }
        pb.finish_with_message("Done!");
      }
    } else if path.exists() && verbose {
      print_message(
        MessageType::Warning,
        format!(
          "File \"{}\" already exists. No need to download it again.",
          path.display()
        )
        .as_str(),
      )
    }
    if md5 && !self.check_hash(path, force, no_download, verbose) {
      self
        .download_file(force, md5, verbose, no_download, total_files)
        .await;
    }
  }

  fn check_hash(&mut self, path: &Path, force: bool, no_download: bool, verbose: bool) -> bool {
    if let Some(correct_hash) = &self.md5Checksum {
      let mut downloaded_file = File::open(path).unwrap();
      let mut hasher = md5::Context::new();
      io::copy(&mut downloaded_file, &mut hasher).unwrap();
      let hash = hasher.compute();
      if format!("{:x}", hash) != *correct_hash {
        if force && !no_download {
          print_message(
            MessageType::Warning,
            format!(
              "MD5 checksum for \"{}\" does NOT match. Downloading file again...",
              path.display()
            )
            .as_str(),
          );
          fs::remove_file(path).unwrap();
          return false;
        } else {
          print_message(
            MessageType::Error,
            format!("MD5 checksum for \"{}\" does NOT match.", path.display()).as_str(),
          );
        }
      } else if verbose {
        print_message(
          MessageType::Info,
          format!(
            "The MD5 hash of \"{}\" matches the original.",
            path.display()
          )
          .as_str(),
        );
      }
    } else {
      print_message(
        MessageType::Warning,
        "Can't check file-integrity. Google didn't provide an md5 hash :/",
      );
    }
    true
  }

  fn create_path(&mut self, verbose: bool) {
    let path_str = if self.mimeType == "application/vnd.google-apps.folder" {
      format!("{}/{}", self.path.clone().unwrap(), self.title)
    } else {
      self.path.clone().unwrap().to_string()
    };
    let path: &Path = Path::new(&path_str);
    if path.exists() {
      if verbose {
        print_message(
          MessageType::Info,
          format!(
            "Folder \"{}\" already exists. No need to create it.",
            path.display()
          )
          .as_str(),
        )
      }
    } else if create_dir_all(path).is_ok() {
      print_message(
        MessageType::Info,
        format!("Created folder \"{}\".", path.display()).as_str(),
      );
    }
  }
}

pub async fn download_folder(
  google_page: GooglePage,
  force: bool,
  recursive: bool,
  md5: bool,
  verbose: bool,
  no_download: bool,
) {
  let mut total_files = GooglePage { items: vec![] };
  for mut item in google_page.items {
    if recursive {
      item
        .download_content(force, md5, verbose, no_download, &mut total_files)
        .await;
    } else if item.mimeType != "application/vnd.google-apps.folder" {
      item
        .download_file(force, md5, verbose, no_download, &mut total_files)
        .await;
    }
  }
  if no_download && !total_files.items.is_empty() {
    print_message(
      MessageType::Info,
      format!("Would've downloaded {} file(s):", total_files.items.len()).as_str(),
    );
    println!("{}", total_files);
  }
}
