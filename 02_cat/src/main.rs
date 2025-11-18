use std::env;
use std::f32::consts::E;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    // 第一步：获取你那个狗屎一样的目标目录


    let target_dir = Path::new("D:\\download\\");
  

    // 确保你给的路径是个目录，而不是你妈的裤衩
    if !target_dir.is_dir() {
        eprintln!("错误: '{}' 不是一个有效的目录。", target_dir.display());
        return;
    }

    println!("开始操翻目录: {}", target_dir.display());

    // 第二步：遍历这个目录，看看里面都有些什么垃圾
    match fs::read_dir(target_dir) {
        Ok(entries) => {
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    // 确保我们只处理文件，别他妈把文件夹也给移了
                    if path.is_file() {
                        classify_and_move_file(&path, target_dir);
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("错误: 无法读取目录 '{}': {}", target_dir.display(), e);
        }
    }

    println!("搞定收工，你这个废物。");
}

fn classify_and_move_file(file_path: &PathBuf, base_dir: &Path) {
    // 第三步：审问文件，看它是什么扩展名
    if let Some(extension) = file_path.extension().and_then(|s| s.to_str()) {
        let dest_dir = base_dir.join(extension);

        // 第四步：创建分类目录，把它关进去
        if !dest_dir.exists() {
            println!("创建新目录: {}", dest_dir.display());
            if let Err(e) = fs::create_dir_all(&dest_dir) {
                eprintln!("错误: 无法创建目录 '{}': {}", dest_dir.display(), e);
                return;
            }
        }

        // 准备好新的文件路径
        if let Some(file_name) = file_path.file_name() {
            let dest_path = dest_dir.join(file_name);
            println!("移动 {} -> {}", file_path.display(), dest_path.display());

            // 执行移动
            if let Err(e) = fs::rename(file_path, &dest_path) {
                eprintln!("错误: 无法移动文件 '{}': {}", file_path.display(), e);
            }
        }
    } else {
        // 对那些没有扩展名的杂种，单独处理
        println!("警告: 文件 '{}' 没有扩展名，跳过。", file_path.display());
    }
}