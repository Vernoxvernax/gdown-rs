use regex::Regex;
use serde_derive::{Deserialize, Serialize};

use crate::web::{get_drive_files, get_drive_html};

#[derive(Serialize, Deserialize, Debug)]
pub struct GooglePage {
  pub items: Vec<GoogleItem>
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug)]
pub struct GoogleItem {
  pub id: String,
  pub title: String,
  pub mimeType: String,
  pub md5Checksum: Option<String>,
  pub downloadUrl: Option<String>,
  pub fileSize: Option<String>,
  pub children: Option<Vec<GoogleItem>>,
  pub path: Option<String>
}

impl GoogleItem {
  fn retrieve_children(&mut self, key: &str, path: String) {
    if self.mimeType == "application/vnd.google-apps.folder" {
      let inner_files = get_drive_files(&self.id, key).unwrap();
      let mut children = Vec::new();
      for mut inner_item in inner_files.items {
        inner_item.retrieve_children(key, format!("{}/{}", path, self.title));
        children.push(inner_item);
      }
      self.children = Some(children);
      self.path = Some(path);
    } else {
      self.path = Some(path);
    }
  }
}

pub fn process_folder_id(id: &str) -> Result<GooglePage, ()> {
  let drive_html = get_drive_html(id).unwrap();

  // Regex to get the key
  let reg = Regex::new("(?:__initData.*?)(?:[a-zA-Z0-9]{39}.*?)([a-zA-Z0-9]{39})").unwrap();
  let capts = reg.captures(&drive_html).unwrap();
  let key = capts.get(1).unwrap().as_str();

  // Get the actual files as json
  let mut json_page = get_drive_files(id, key).unwrap();

  // Resolve folders
  for json_item in &mut json_page.items {
    json_item.retrieve_children(key, String::from("."));
  }
  
  Ok(json_page)
}
