use reqwest::{
  StatusCode, {Client, Response},
};
use urlencoding::encode;

use crate::google_drive::GooglePage;

const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0.0.0 Safari/537.36";

pub async fn api_post(url: String, body: String) -> Result<String, ()> {
  let client = Client::new();
  let request = client
    .post(url)
    .header("Origin", "https://drive.google.com")
    .header("Content-Type", "text/plain")
    .header("User-Agent", USER_AGENT)
    .body(body)
    .send()
    .await
    .unwrap();

  match request.status() {
    StatusCode::OK => Ok(request.text().await.unwrap()),
    _ => {
      eprintln!("{}", request.text().await.unwrap());
      Err(())
    },
  }
}

pub async fn api_get(url: String) -> Result<String, ()> {
  let client = Client::new();
  let request = client
    .get(url)
    .header("Content-Type", "text/plain")
    .header("User-Agent", USER_AGENT)
    .send()
    .await
    .unwrap();

  match request.status() {
    StatusCode::OK => Ok(request.text().await.unwrap()),
    _ => {
      eprintln!("{}", request.text().await.unwrap());
      Err(())
    },
  }
}

pub async fn api_get_file(id: String) -> Result<Response, ()> {
  let client = Client::new();
  let request = client
    .get(format!(
      "https://drive.usercontent.google.com/download?id={}&export=download&confirm=t",
      id
    ))
    .header("Origin", "https://drive.google.com")
    .header("User-Agent", USER_AGENT)
    .send()
    .await
    .unwrap();

  match request.status() {
    StatusCode::OK => Ok(request),
    _ => {
      eprintln!("{}", request.text().await.unwrap());
      Err(())
    },
  }
}

fn get_json_part(plain: String) -> String {
  let mut json = String::new();
  let mut curly_layers = 0;
  for ch in plain.chars() {
    if ch == '{' {
      curly_layers += 1;
    }
    if curly_layers >= 1 {
      json.push(ch);
    }
    if ch == '}' {
      curly_layers -= 1;
    }
  }
  json
}

pub async fn get_drive_files(id: &str, key: &str) -> Result<GooglePage, ()> {
  let boundary: &str = "=====vc17a3rwnndj=====";
  let ct: &str = &("multipart/mixed; boundary=\"".to_owned() + boundary + "\"");
  let body = format!(
    "--{}
content-type: application/http
content-transfer-encoding: binary

GET /drive/v2beta/files?q=trashed%20%3D%20false%20and%20'{}'%20in%20parents&key={} HTTP/1.1

--{}--",
    boundary, id, key, boundary
  );

  match api_post(
    format!(
      "https://clients6.google.com/batch/drive/v2beta?{}={}",
      encode("$ct"),
      encode(ct)
    ),
    body,
  )
  .await
  {
    Ok(res) => {
      let plain_json = get_json_part(res);
      Ok(serde_json::from_str::<GooglePage>(&plain_json).unwrap())
    },
    _ => Err(()),
  }
}

pub async fn get_drive_html(id: &str) -> Result<String, ()> {
  api_get(format!("https://drive.google.com/drive/folders/{}", id)).await
}
