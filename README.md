# GDown-RS

Google-Drive CLI Downloader written in Rust.

___

**Usage:**
```
gdown <folder-id>
```

+ `--force` overwrite existing files
+ `--check` use the md5 hash to check for file-integrity 
+ `--non-recursively` only download files in the root directory
+ `--verbose` print all warning messages
+ `--no-download` don't do any local changes (if you're paranoid to loose something; it's very verbose)
+ `--output-folder` instead of <folder-id>, use this as the root folder name
+ `--file-id` there is no support for individual files. Use wget

___

This project is very new so there might be problems. Like f.e. rate-limits are currently not handled at all.

