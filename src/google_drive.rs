use async_trait::async_trait;
use regex::Regex;
use serde_derive::{Deserialize, Serialize};
use std::fmt;

use crate::{
  print_message,
  web::{get_drive_files, get_drive_html},
  MessageType,
};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GooglePage {
  pub items: Vec<GoogleItem>,
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GoogleItem {
  pub id: String,
  pub title: String,
  pub mimeType: String,
  pub md5Checksum: Option<String>,
  pub downloadUrl: Option<String>,
  pub fileSize: Option<String>,
  pub children: Option<Vec<GoogleItem>>,
  pub path: Option<String>,
}

#[async_trait]
pub trait RetrieveChildren {
  async fn retrieve_children(&mut self, key: &str, path: String, verbose: bool);
}

#[async_trait]
impl RetrieveChildren for GoogleItem {
  async fn retrieve_children(&mut self, key: &str, path: String, verbose: bool) {
    if self.mimeType == "application/vnd.google-apps.folder" {
      if verbose {
        print_message(
          MessageType::Info,
          "GET: JSON for files and folders in subdirectory.",
        );
      }
      let inner_files = get_drive_files(&self.id, key).await.unwrap();
      let mut children = Vec::new();
      for mut inner_item in inner_files.items {
        inner_item
          .retrieve_children(key, format!("{}/{}", path, self.title), verbose)
          .await;
        children.push(inner_item);
      }
      self.children = Some(children);
      self.path = Some(path);
    } else {
      self.path = Some(path);
    }
  }
}

impl fmt::Display for GooglePage {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "[")?;
    for (index, item) in self.items.iter().enumerate() {
      write!(f, "{}", item.title)?;
      if index + 1 != self.items.len() {
        write!(f, ", ")?;
      }
    }
    write!(f, "]")
  }
}

pub async fn process_folder_id(
  id: &str,
  output_folder: &String,
  verbose: bool,
) -> Result<GooglePage, ()> {
  if verbose {
    print_message(MessageType::Info, "GET: HTML from Google Drive folder.");
  }

  let drive_html = get_drive_html(id).await.unwrap();

  // Regex to get the key
  let reg = Regex::new("(?:__initData.*?)(?:[a-zA-Z0-9]{39}.*?)([a-zA-Z0-9]{39})").unwrap();
  let capts = reg.captures(&drive_html).unwrap();
  let key = capts.get(1).unwrap().as_str();

  // Get the actual files as json
  if verbose {
    print_message(
      MessageType::Info,
      "GET: JSON for files and folders in the root directory.",
    );
  }
  let mut json_page = get_drive_files(id, key).await.unwrap();

  // Resolve folders
  for json_item in &mut json_page.items {
    json_item
      .retrieve_children(key, format!("./{}", output_folder), verbose)
      .await;
  }

  Ok(json_page)
}
