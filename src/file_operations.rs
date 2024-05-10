use std::{fs, path::Path};

pub fn list_files(path: &Path) -> fs::ReadDir {
    fs::read_dir(path).unwrap()
}

pub fn show_dir_size(path: &Path) -> u64 {
    let mut size = 0;
    if path.is_dir() {
        let paths = fs::read_dir(path).unwrap();
        for path in paths {
            let path = path.unwrap().path();
            let metadata: fs::Metadata = fs::metadata(&path).unwrap();
            if metadata.is_dir() {
                size += show_dir_size(&path);
            } else {
                size += metadata.len();
            }
        }
    }
    size
}

pub fn get_file_metadata(path: &Path) -> fs::Metadata {
    fs::metadata(path).unwrap()
}

pub fn remove_file_if(path: &Path, condition: impl Fn(&str) -> bool) {
    let file_name = path.file_name().unwrap().to_str().unwrap();
    if condition(file_name) {
        match fs::remove_file(path) {
            Ok(_) => println!("文件删除成功"),
            Err(err) => eprintln!("Error removing file: {}", err),
        }
    }
}
// use std::path::Path;
// use chrono::NaiveDateTime;
// use file_operations::*;
// let path = Path::new("C:\\Users\\83795\\Downloads");
// let paths = list_files(&path);
// let size = show_dir_size(&path);
// println!("文件夹大小: {} mb=======", size / 1024 / 1024);

// for path in paths {
//     let path = path.unwrap().path();
//     let metadata = get_file_metadata(&path);
//     let file_size = metadata.len() / 1024;
//     let modified = NaiveDateTime::from_timestamp(metadata.modified().unwrap().duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap().as_secs() as i64, 0);
//     let created = NaiveDateTime::from_timestamp(metadata.created().unwrap().duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap().as_secs() as i64, 0);

//     remove_file_if(&path, |file_name| file_name.ends_with(".sh"));
// }

// let size = show_dir_size(&path);
// println!("删除后文件夹大小:======= {} mb", size / 1024 / 1024);