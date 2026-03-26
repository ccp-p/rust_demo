use std::fs;
use std::path::PathBuf;
use clap::{Parser};
use image::io::Reader as ImageReader;

// ================== 配置常量 ==================
const HOME_SOURCE_PATH: &str = "D:\\job_project\\china_mobile\\gitProject\\richinfo_tyjf_xhmqqthy\\src\\main\\webapp\\res\\wap";
const COMPANY_SOURCE_PATH: &str = "D:\\project\\cx_project\\china_mobile\\gitProject\\richinfo_tyjf_xhmqqthy\\src\\main\\webapp\\res\\wap";
const COMPRESSED_PATH: &str = "C:\\Users\\83795\\Downloads\\compressed";
const DATE_DIR: &str = "202505";

// ================== 主逻辑 ==================
#[derive(Parser)]
struct Args {
    #[arg(short, long)]
    home: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // 1. 确定基础路径
    let base_path = if args.home {
        PathBuf::from(HOME_SOURCE_PATH)
    } else {
        PathBuf::from(COMPANY_SOURCE_PATH)
    };

    println!("[开始处理] 当前环境: {}", if args.home { "家里电脑" } else { "公司电脑" });
    println!("[基础路径] {}", base_path.display());

    // 2. 读取压缩目录
    let entries = fs::read_dir(COMPRESSED_PATH)?;
    let mut css_groups: std::collections::HashMap<String, Vec<ResourceInfo>> = std::collections::HashMap::new();
    let mut count = 0;

    for entry in entries {
        let file = entry?;
        let path = file.path();
        let name = file.file_name().to_string_lossy().to_string();

        // 忽略非文件或非 xdr 开头
        if !file.file_type()?.is_file() || !name.starts_with("xdr") {
            continue;
        }

        let info = prepare_resource_info(&path, &name, &base_path)?;
        if let Some(info) = info {
            println!("[命中] 文件: {} ({}x{}), Category: {}", info.file_name, info.width, info.height, info.category);
            let css_path = info.css_path.to_string_lossy().to_string();
            css_groups.entry(css_path).or_insert_with(Vec::new).push(info);
            count += 1;
        } else {
            println!("[跳过] 文件: {}", name);
        }
    }

    // 3. 批量更新每个 CSS
    for (css_path, infos) in css_groups {
        update_batch(&css_path, &infos, &base_path)?;
    }
  // 阻止窗口立即关闭
  
  println!("[完成] 共处理 {} 个文件。", count);

  println!("处理完成，按下任意键（例如空格）退出...");
    console::Term::stdout().read_key()?;
    
   

    Ok(())
}

// ================== 资源信息准备 ==================
fn prepare_resource_info(
    file_path: &PathBuf,
    file_name: &str,
    base_path: &PathBuf,
) -> Result<Option<ResourceInfo>, Box<dyn std::error::Error>> {
    let (width, height) = get_image_dimension(file_path)?;

    let name_only = PathBuf::from(file_name)
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let (category, target_dir, css_path) = if (width as i32 - 220).abs() <= 10 && (height as i32 - 220).abs() <= 10 {
        ("popQy", base_path.join("components/xdrsign/static/popQy"), base_path.join("components/xdrsign/index.css"))
    } else if (width as i32 - 200).abs() <= 10 && (height as i32 - 208).abs() <= 10 {
        ("signPrize", base_path.join("components/xdrsign/static"), base_path.join("components/xdrsign/index.css"))
    } else {
        let cat = if name_only.ends_with("_not_start") {
            "notStart"
        } else if name_only.contains("_xdr") {
            "xdrPrize"
        } else {
            "normalPrize"
        };
        (cat, base_path.join(format!("images/xdrNormal/{}", DATE_DIR)), base_path.join("css/xdrNormal.css"))
    };

    let css_content = generate_css_content(category, &name_only, file_name)?;

    let info = ResourceInfo {
        file_path: file_path.clone(),
        file_name: file_name.to_string(),
        name_only,
        width,
        height,
        category: category.to_string(),
        target_dir,
        css_path,
        css_content,
    };

    Ok(Some(info))
}

// ================== 图像尺寸获取 ==================
fn get_image_dimension(path: &PathBuf) -> Result<(u32, u32), Box<dyn std::error::Error>> {
    let reader = ImageReader::open(path)?;
    let dimensions = reader.into_dimensions()?;
    Ok(dimensions)
}

// ================== 生成 CSS 内容 ==================
fn generate_css_content(
    category: &str,
    name_only: &str,
    file_name: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    match category {
        "popQy" => Ok(format!(
            ".level-sign-popup .level-sign-popup-prize.{} {{\n    background-image: url('../../components/xdrsign/static/popQy/{}');\n}}\n",
            name_only, file_name
        )),
        "signPrize" => Ok(format!(
            ".level-sign-prize-wrapper #level-sign-prize-swiper .swiper-slide.{} {{\n    background-image: url('../../components/xdrsign/static/{}');\n}}\n",
            name_only, file_name
        )),
        _ => {
            let content = if let Some(before) = name_only.strip_suffix("_not_start") {
                format!(".level-award-center #XdrNotStartList #not-start-swiper .swiper-slide.{} {{\n  background-image: url('../images/xdrNormal/{}/{}');\n}}\n", before, DATE_DIR, file_name)
            } else if let Some(before) = name_only.strip_suffix("_xdr_r") {
                format!(".level-award-center #XdrPrizeList .level-award-prize .item.{}.received {{\n  background-image: url('../images/xdrNormal/{}/{}');\n}}\n", before, DATE_DIR, file_name)
            } else if let Some(before) = name_only.strip_suffix("_r") {
                format!(".level-award-prize .item.{}.received {{\n    background-image: url('../images/xdrNormal/{}/{}');\n}}\n", before, DATE_DIR, file_name)
            } else if let Some(before) = name_only.strip_suffix("_xdr") {
                format!(".level-award-center #XdrPrizeList .level-award-prize .item.{} {{\n  background-image: url('../images/xdrNormal/{}/{}');\n}}\n", before, DATE_DIR, file_name)
            } else {
                format!("/* {} */\n.level-award-prize .item.{} {{\n    background-image: url('../images/xdrNormal/{}/{}');\n}}\n", name_only, name_only, DATE_DIR, file_name)
            };
            Ok(content)
        }
    }
}

// ================== 批量更新 CSS ==================
fn update_batch(css_path: &str, infos: &[ResourceInfo], _base_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let mut current_content = fs::read_to_string(css_path)?;
    let mut updated = false;

    for info in infos {
        fs::create_dir_all(&info.target_dir)?;
        let target_path = info.target_dir.join(&info.file_name);
        fs::copy(&info.file_path, &target_path)?;
        fs::remove_file(&info.file_path)?;

        let selector_line = info.css_content.lines()
            .find(|l| l.contains('{'))
            .unwrap_or(&info.name_only);
        let check_str = selector_line.split('{').next().unwrap_or(selector_line).trim();

        if current_content.contains(check_str) {
            continue;
        }

        let insert_pos = find_insert_position(&current_content, &info.category)?;
        current_content.insert_str(insert_pos, &format!("\n{}", info.css_content));
        updated = true;
    }

    if updated {
        fs::write(css_path, current_content)?;
    }
    
    let display_name = PathBuf::from(css_path).file_name().unwrap_or_default().to_string_lossy().to_string();
    println!("[CSS] 已处理: {}", display_name);
    Ok(())
}

// ================== 查找插入位置 ==================
fn find_insert_position(content: &str, category: &str) -> Result<usize, Box<dyn std::error::Error>> {
    let selector = match category {
        "popQy" => ".level-sign-popup .level-sign-popup-prize",
        "signPrize" => ".level-sign-prize-wrapper #level-sign-prize-swiper .swiper-slide",
        "notStart" => "#XdrNotStartList #not-start-swiper .swiper-slide",
        "xdrPrize" => "#XdrPrizeList .level-award-prize .item",
        "normalPrize" => ".level-award-prize .item",
        _ => "",
    };

    if selector.is_empty() {
        return Ok(content.len());
    }

    match content.rfind(selector) {
        Some(last_idx) => {
            let suffix = &content[last_idx..];
            match suffix.find('}') {
                Some(end_brace) => Ok(last_idx + end_brace + 1),
                None => Ok(content.len()),
            }
        }
        None => Ok(content.len()),
    }
}

// ================== 资源信息结构体 ==================
#[derive(Debug)]
struct ResourceInfo {
    file_path: PathBuf,
    file_name: String,
    name_only: String,
    width: u32,
    height: u32,
    category: String,
    target_dir: PathBuf,
    css_path: PathBuf,
    css_content: String,
}