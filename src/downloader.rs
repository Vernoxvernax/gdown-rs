use std::{fs::{self, create_dir_all, File}, io::{self, Read, Write}, path::Path, thread, time::Duration};

use indicatif::{FormattedDuration, HumanBytes, ProgressBar, ProgressStyle};
use isahc::ResponseExt;

use crate::{google_drive::{GoogleItem, GooglePage}, print_warning_message, web::api_get_file};

impl GoogleItem {
  fn download_content(&mut self, force: bool, md5: bool, verbose: bool) {
    if self.mimeType == "application/vnd.google-apps.folder" {
      create_folder(self, verbose);
      if let Some(children) = &mut self.children {
        for file in children.iter_mut() {
          file.download_content(force, md5, verbose);
        }
      }
    } else {
      download_file(self, force, md5, verbose);
    }
  }
}

fn create_folder(folder: &GoogleItem, verbose: bool) {
  let path_str = format!("{}/{}", folder.path.clone().unwrap(), folder.title);
  let path: &Path = Path::new(&path_str);
  if path.exists() {
    if verbose {
      print_warning_message(format!("Folder \"{}\" already exists. No need to create it.", path.display()).as_str())
    }
  } else if create_dir_all(path).is_ok() {
    print_warning_message(format!("Created folder \"{}\".", path.display()).as_str());
  }
}

fn download_file(file: &GoogleItem, force: bool, md5: bool, verbose: bool) {
  let path_str = format!("{}/{}", file.path.clone().unwrap(), file.title);
  let path: &Path = Path::new(&path_str);
  if !path.exists() || force {
    print_warning_message(format!("Starting downloading for file: \"{}\"", path.display()).as_str());
    thread::sleep(Duration::from_millis(250));
    let total_size = file.fileSize.clone().unwrap().parse().unwrap();
    let pb = ProgressBar::new(total_size);
    pb.set_style(
      ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})")
      .unwrap()
    .progress_chars("#>-"));

    if let Ok(mut stream) = api_get_file(file.id.clone()) {
      let mut dest = File::create(path).unwrap();
  
      let metrics = stream.metrics().unwrap().clone();
      let body = stream.body_mut();
      let mut buf = [0; 16384 * 4];

      loop {
        match body.read(&mut buf) {
          Ok(0) => {
            pb.finish_with_message("downloaded");
            break;
          },
          Ok(bytes_read) => {
            pb.set_position(metrics.download_progress().0);
            pb.set_length(metrics.download_progress().1);
            pb.set_message(format!(
              "time: {}  speed: {}/sec",
              FormattedDuration(metrics.total_time()),
              HumanBytes(metrics.download_speed() as u64),
            ));
            dest.write_all(&buf[..bytes_read]).unwrap();
          },
          Err(e) => {
            pb.finish();
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
    if let Some(correct_hash) = &file.md5Checksum {
      let mut downloaded_file = File::open(path).unwrap();
      let mut hasher = md5::Context::new();
      io::copy(&mut downloaded_file, &mut hasher).unwrap();
      let hash = hasher.compute();
      if format!("{:x}", hash) != *correct_hash {
        if force {
          print_warning_message(format!("MD5 checksum for \"{}\" does NOT match. Deleting file...", path.display()).as_str());
          fs::remove_file(path).unwrap();
          download_file(file, force, md5, verbose);
        } else {
          print_warning_message(format!("MD5 checksum for \"{}\" does NOT match.", path.display()).as_str());
        }
      }
    }
  }
}

pub fn download_folder(google_page: GooglePage, force: bool, recursive: bool, md5: bool, verbose: bool) {
  for mut item in google_page.items {
    if recursive {
      item.download_content(force, md5, verbose);
    } else if item.mimeType != "application/vnd.google-apps.folder" {
      download_file(&item, force, md5, verbose);
    }
  }
}
