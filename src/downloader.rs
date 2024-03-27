use std::{fs::{self, create_dir_all, File}, io::{self, Read, Write}, path::Path, thread, time::Duration};

use indicatif::{HumanBytes, ProgressBar, ProgressStyle};
use isahc::ResponseExt;

use crate::{google_drive::{GoogleItem, GooglePage}, print_warning_message, web::api_get_file};

impl GoogleItem {
  fn download_content(&mut self, force: bool, md5: bool, verbose: bool, no_download: bool, total_files: &mut GooglePage) {
    if self.mimeType == "application/vnd.google-apps.folder" {
      if let Some(children) = &mut self.children {
        for file in children.iter_mut() {
          file.download_content(force, md5, verbose, no_download, total_files);
        }
      }
    } else {
      self.download_file(force, md5, verbose, no_download, total_files);
    }
  }

  fn download_file(&mut self, force: bool, md5: bool, verbose: bool, no_download: bool, total_files: &mut GooglePage) {
    if !no_download {
      create_path(self, verbose);
    }
    let path_str = format!("{}/{}", self.path.clone().unwrap(), self.title);
    let path: &Path = Path::new(&path_str);
    if !path.exists() || force {
      if no_download {
        total_files.items.push(self.clone());
        return;
      }
      print_warning_message(format!("Starting downloading for file: \"{}\"", path.display()).as_str());
      thread::sleep(Duration::from_millis(250));
  
      let pb = ProgressBar::new_spinner().with_style(
        ProgressStyle::with_template("[{spinner:.blue}]-[{wide_bar:.blue/grey}] [{bytes}/{total_bytes}] {msg} ({eta})")
          .unwrap()
          .tick_chars("-\\|/!")
          .progress_chars("=>-")
      );
      pb.enable_steady_tick(Duration::from_millis(100));
  
      if let Ok(mut stream) = api_get_file(self.id.clone()) {
        let mut dest = File::create(path).unwrap();
    
        let metrics = stream.metrics().unwrap().clone();
        let body = stream.body_mut();
        let mut buf = [0; 16384 * 4];
  
        loop {
          match body.read(&mut buf) {
            Ok(0) => {
              pb.finish_with_message("Done!");
              break;
            },
            Ok(bytes_read) => {
              pb.set_position(metrics.download_progress().0);
              pb.set_length(metrics.download_progress().1);
              pb.set_message(format!(
                "[{}/s]",
                // FormattedDuration(metrics.total_time()), // maybe in the future
                HumanBytes(metrics.download_speed() as u64),
              ));
              dest.write_all(&buf[..bytes_read]).unwrap();
            },
            Err(e) => {
              pb.finish_with_message("Error!");
              eprintln!("Error: {}", e);
              break;
            }
          }
        }
      }
    } else if path.exists() && verbose {
      print_warning_message(format!("File \"{}\" already exists. No need to download it again.", path.display()).as_str())
    }
    if md5 {
      if !self.check_hash(path, force, no_download) {
        self.download_file(force, md5, verbose, no_download, total_files);
      }
    }
  }
  
  fn check_hash(&mut self, path: &Path, force: bool, no_download: bool) -> bool {
    if let Some(correct_hash) = &self.md5Checksum {
      let mut downloaded_file = File::open(path).unwrap();
      let mut hasher = md5::Context::new();
      io::copy(&mut downloaded_file, &mut hasher).unwrap();
      let hash = hasher.compute();
      if format!("{:x}", hash) != *correct_hash {
        if force && !no_download {
          print_warning_message(format!("MD5 checksum for \"{}\" does NOT match. Deleting file...", path.display()).as_str());
          fs::remove_file(path).unwrap();
          return false;
        } else {
          print_warning_message(format!("MD5 checksum for \"{}\" does NOT match.", path.display()).as_str());
        }
      } else {
        print_warning_message(format!("The MD5 hash of \"{}\" matches the original.", path.display()).as_str());
      }
    } else {
      print_warning_message("Can't check file-integrity. Google didn't provide an md5 hash :/");
    }
    true
  }
}

fn create_path(item: &GoogleItem, verbose: bool) {
  let path_str = if item.mimeType == "application/vnd.google-apps.folder" {
    format!("{}/{}", item.path.clone().unwrap(), item.title)
  } else {
    item.path.clone().unwrap().to_string()
  };
  let path: &Path = Path::new(&path_str);
  if path.exists() {
    if verbose {
      print_warning_message(format!("Folder \"{}\" already exists. No need to create it.", path.display()).as_str())
    }
  } else if create_dir_all(path).is_ok() {
    print_warning_message(format!("Created folder \"{}\".", path.display()).as_str());
  }
}


pub fn download_folder(google_page: GooglePage, force: bool, recursive: bool, md5: bool, verbose: bool, no_download: bool) {
  let mut total_files: GooglePage = GooglePage {
    items: vec![]
  };
  for mut item in google_page.items {
    if recursive {
      item.download_content(force, md5, verbose, no_download, &mut total_files);
    } else if item.mimeType != "application/vnd.google-apps.folder" {
      item.download_file(force, md5, verbose, no_download, &mut total_files);
    }
  }
  if no_download && total_files.items.len() > 0 {
    print_warning_message(format!("Would've downloaded {} file(s):", total_files.items.len()).as_str());
    println!("{}", total_files);
  }
}
