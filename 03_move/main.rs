use std::collections::HashMap;
use std::io::Read;
use std::env;
use std::sync::{Arc, Mutex};
use regex::Regex;
use serde::{Deserialize, Serialize};
use clap::Parser;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Config {
    #[serde(default = "default_root_dir")]
    root_dir: String,
    #[serde(default)]
    cdn_domain: String,
    #[serde(default = "default_hash_length")]
    hash_length: usize,
    #[serde(default)]
    single_html_file: String,
    #[serde(default)]
    html_files: Vec<String>,
    #[serde(default = "default_exclude_dirs")]
    exclude_dirs: Vec<String>,
    #[serde(default)]
    home_html_file: String,
    #[serde(default)]
    company_html_file: String,
    #[serde(default)]
    include_components: Vec<String>,
}

fn default_root_dir() -> String { ".".to_string() }
fn default_hash_length() -> usize { 8 }
fn default_exclude_dirs() -> Vec<String> { vec!["node_modules".to_string(), ".git".to_string(), "dist".to_string(), "build".to_string()] }

#[derive(Debug)]
struct FileInfo {
    original_path: String,
    hashed_path: String,
    hash: String,
    renamed: bool,
}

#[derive(Debug)]
struct ImageReference {
    original_path: String,
    absolute_path: String,
    relative_path: String,
}

struct VersionManager {
    config: Config,
    version_map: Arc<Mutex<HashMap<String, String>>>,
    processed_files: Arc<Mutex<HashMap<String, bool>>>,
    debug_mode: bool,
}

impl VersionManager {
    fn new(config: Config, debug_mode: bool) -> Self {
        VersionManager {
            version_map: Arc::new(Mutex::new(HashMap::new())),
            processed_files: Arc::new(Mutex::new(HashMap::new())),
            config,
            debug_mode,
        }
    }

    fn should_process_component(&self, component_path: &str) -> bool {
        if self.config.include_components.is_empty() {
            return true;
        }

        for component_name in &self.config.include_components {
            if component_path.contains(&format!("/{}/", component_name)) ||
               component_path.contains(&format!("\\{}\\", component_name)) ||
               component_path.ends_with(&format!("/{}", component_name)) ||
               component_path.ends_with(&format!("\\{}", component_name)) ||
               std::path::Path::new(component_path).file_stem()
                   .map(|s| s.to_string_lossy().starts_with(&format!("{}.0", component_name))) // 假设文件名格式
                   .unwrap_or(false) {
                return true;
            }
        }

        false
    }

    fn calculate_file_hash(&self, file_path: &str) -> Result<String, Box<dyn std::error::Error>> {
        let mut file = std::fs::File::open(file_path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;

        let hash = format!("{:x}", md5::compute(&buffer));
        
        if self.config.hash_length > 0 && self.config.hash_length < hash.len() {
            Ok(hash[..self.config.hash_length].to_string())
        } else {
            Ok(hash)
        }
    }

    fn remove_hash_from_filename(&self, filename: &str) -> String {
        let re = Regex::new(r"^(.+)\.([a-f0-9]{8})\.(css|js|jpg|jpeg|png|gif|svg|webp|ico)$").unwrap();
        if let Some(caps) = re.captures(filename) {
            return format!("{}.{}", &caps[1], &caps[3]);
        }
        filename.to_string()
    }

    fn add_hash_to_filename(&self, filename: &str, hash: &str) -> String {
        let path = std::path::Path::new(filename);
        let ext = path.extension().map(|e| e.to_string_lossy()).unwrap_or_default();
        let basename = path.file_stem().map(|b| b.to_string_lossy()).unwrap_or_default();
        
        let re = Regex::new(r"\.[a-f0-9]{8}$").unwrap();
        let clean_basename = re.replace(&basename, "");
        
        if ext.is_empty() {
            format!("{}.{}", clean_basename, hash)
        } else {
            format!("{}.{}.{}", clean_basename, hash, ext)
        }
    }

    fn find_and_delete_old_hash_files(&self, dir: &str, basename: &str, ext: &str, current_hash: &str) -> Result<(), Box<dyn std::error::Error>> {
        if self.debug_mode {
            println!("  🔍 查找旧hash文件: {}{} (当前hash: {})", basename, ext, current_hash);
        }

        let pattern = format!(r"^{}\.[a-f0-9]{{8}}{}$", regex::escape(basename), regex::escape(ext));
        let re = Regex::new(&pattern)?;

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let filename = entry.file_name().to_string_lossy().to_string();

            if re.is_match(&filename) {
                let expected_pattern = format!(r"^{}\.(.{{8}}){}$", regex::escape(basename), regex::escape(ext));
                let hash_re = Regex::new(&expected_pattern)?;
                
                if let Some(caps) = hash_re.captures(&filename) {
                    let extracted_hash = &caps[1];
                    
                    if extracted_hash != current_hash {
                        let old_file_path = std::path::Path::new(dir).join(&filename);
                        std::fs::remove_file(&old_file_path)?;
                        println!("    🗑️  已删除: {}", filename);
                    }
                }
            }
        }

        Ok(())
    }

    fn rename_file_with_hash(&self, file_path: &str) -> Result<FileInfo, Box<dyn std::error::Error>> {
        let path = std::path::Path::new(file_path);
        let dir = path.parent().unwrap().to_string_lossy().to_string();
        let filename = path.file_name().unwrap().to_string_lossy().to_string();
        let clean_filename = self.remove_hash_from_filename(&filename);

        let clean_path = std::path::Path::new(&dir).join(&clean_filename);
        let source_path = if clean_path.exists() {
            clean_path.to_string_lossy().to_string()
        } else {
            file_path.to_string()
        };

        let hash = self.calculate_file_hash(&source_path)?;
        let new_filename = self.add_hash_to_filename(&clean_filename, &hash);
        let new_path = std::path::Path::new(&dir).join(&new_filename).to_string_lossy().to_string();

        let info = FileInfo {
            original_path: source_path.clone(),
            hashed_path: new_path.clone(),
            hash: hash.clone(),
            renamed: true,
        };

        if std::path::Path::new(&new_path).exists() {
            let existing_hash = self.calculate_file_hash(&new_path)?;
            if existing_hash == hash {
                if self.debug_mode {
                    println!("  ⏭️  跳过（已存在）: {}", new_filename);
                }
                return Ok(info);
            }
            std::fs::remove_file(&new_path)?;
        }

        std::fs::copy(&source_path, &new_path)?;
        println!("  ✅ 已生成: {}", new_filename);

        let ext = std::path::Path::new(&clean_filename).extension().unwrap_or_default().to_string_lossy().to_string();
        let basename = std::path::Path::new(&clean_filename).file_stem().unwrap().to_string_lossy().to_string();
        self.find_and_delete_old_hash_files(&dir, &basename, &ext, &hash)?;

        Ok(info)
    }

    fn collect_images_from_css(&self, css_path: &str) -> Result<Vec<ImageReference>, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(css_path)?;
        let css_dir = std::path::Path::new(css_path).parent().unwrap().to_string_lossy().to_string();
        let mut images = Vec::new();

        let re = Regex::new(r#"url\(['"]?([^'")\s]+)['"]?\)"#)?;
        for cap in re.captures_iter(&content) {
            let image_path = &cap[1];

            if image_path.starts_with("http") || 
               image_path.starts_with("data:") || 
               image_path.starts_with("//") {
                continue;
            }

            let image_path = image_path.split('?').next().unwrap_or(image_path).split('#').next().unwrap_or(image_path);

            let absolute_path = std::path::Path::new(&css_dir).join(std::path::Path::new(&image_path));
            let absolute_path = absolute_path.canonicalize().unwrap_or(absolute_path);
            let absolute_path_str = absolute_path.to_string_lossy().to_string();

            if std::path::Path::new(&absolute_path_str).exists() {
                let relative_path = pathdiff::diff_paths(&absolute_path, &css_dir).unwrap_or_else(|| std::path::Path::new(&image_path).to_path_buf());
                images.push(ImageReference {
                    original_path: image_path.to_string(),
                    absolute_path: absolute_path_str,
                    relative_path: relative_path.to_string_lossy().to_string(),
                });
            }
        }

        Ok(images)
    }

    fn update_css_image_references(&self, css_path: &str, image_map: &HashMap<String, String>) -> Result<(), Box<dyn std::error::Error>> {
        let mut content = std::fs::read_to_string(css_path)?;
        let mut updated = false;

        for (original_path, new_filename) in image_map {
            let old_filename = std::path::Path::new(original_path).file_name().unwrap().to_string_lossy().to_string();
            let clean_old_filename = self.remove_hash_from_filename(&old_filename);

            let pattern = format!(r#"url\(\s*(['"]?)\s*([^'")\s]*[/\\])?{}(?:\s*(['"]?)\s*\))"#, regex::escape(&clean_old_filename));
            let re = Regex::new(&pattern)?;

            content = re.replace_all(&content, |caps: &regex::Captures| {
                let opening_quote = &caps[1];
                let path_prefix = &caps[2];
                let closing_quote = caps.get(3).map(|m| m.as_str()).unwrap_or("");

                let result = format!("url({}{}{}{})", opening_quote, path_prefix, new_filename, closing_quote);

                if &caps[0] != &result {
                    updated = true;
                    println!("    🔄 {} -> {}", clean_old_filename, new_filename);
                }
                result
            }).to_string();
        }

        if updated {
            std::fs::write(css_path, content)?;
        }

        Ok(())
    }

    fn find_file(&self, base_path: &str) -> Option<String> {
        let path = std::path::Path::new(base_path);
        
        if path.exists() {
            return Some(base_path.to_string());
        }

        let dir = path.parent().unwrap().to_string_lossy().to_string();
        let ext = path.extension().unwrap_or_default().to_string_lossy().to_string();
        let name_without_ext = path.file_stem().unwrap().to_string_lossy().to_string();

        let pattern = format!(r"^{}\.[a-f0-9]{{8}}\\{}$", regex::escape(&name_without_ext), regex::escape(&ext));
        let re = Regex::new(&pattern).unwrap();

        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries {
                if let Ok(entry) = entry {
                    let filename = entry.file_name().to_string_lossy().to_string();
                    if re.is_match(&filename) {
                        return Some(std::path::Path::new(&dir).join(&filename).to_string_lossy().to_string());
                    }
                }
            }
        }

        None
    }

    fn collect_resources_from_html(&self, html_path: &str) -> Result<HashMap<String, Vec<String>>, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(html_path)?;
        let html_dir = std::path::Path::new(html_path).parent().unwrap().to_string_lossy().to_string();
        let mut resources = HashMap::new();
        resources.insert("css".to_string(), Vec::new());
        resources.insert("js".to_string(), Vec::new());

        let css_re = Regex::new(r#"<link[^>]*href\s*=\s*['"]([^'"]+\.css)['"]"#)?;
        for cap in css_re.captures_iter(&content) {
            let css_path = &cap[1];

            if css_path.starts_with("http") || css_path.starts_with("//") {
                continue;
            }

            if !css_path.contains("components") {
                continue;
            }

            if !self.should_process_component(css_path) {
                if self.debug_mode {
                    println!("    🚫 跳过组件CSS: {} (不在处理列表中)", css_path);
                }
                continue;
            }

            let absolute_path = std::path::Path::new(&html_dir).join(std::path::Path::new(&css_path));
            let absolute_path = absolute_path.canonicalize().unwrap_or(absolute_path);
            let absolute_path_str = absolute_path.to_string_lossy().to_string();

            if absolute_path.exists() || self.find_file(&absolute_path_str).is_some() {
                if let Some(css_list) = resources.get_mut("css") {
                    css_list.push(css_path.to_string());
                    println!("    📌 收集组件CSS: {}", css_path);
                }
            }
        }

        let js_re = Regex::new(r#"<script[^>]*src\s*=\s*['"]([^'"]+\.js)['"]"#)?;
        for cap in js_re.captures_iter(&content) {
            let js_path = &cap[1];

            if js_path.starts_with("http") || js_path.starts_with("//") {
                continue;
            }

            if !js_path.contains("components") {
                continue;
            }

            if !self.should_process_component(js_path) {
                if self.debug_mode {
                    println!("    🚫 跳过组件JS: {} (不在处理列表中)", js_path);
                }
                continue;
            }

            let absolute_path = std::path::Path::new(&html_dir).join(std::path::Path::new(&js_path));
            let absolute_path = absolute_path.canonicalize().unwrap_or(absolute_path);
            let absolute_path_str = absolute_path.to_string_lossy().to_string();

            if absolute_path.exists() || self.find_file(&absolute_path_str).is_some() {
                if let Some(js_list) = resources.get_mut("js") {
                    js_list.push(js_path.to_string());
                    println!("    📌 收集组件JS: {}", js_path);
                }
            }
        }

        Ok(resources)
    }

    fn process_component_resource(&self, html_dir: &str, relative_path: &str) -> Result<FileInfo, Box<dyn std::error::Error>> {
        let absolute_path = std::path::Path::new(html_dir).join(std::path::Path::new(&relative_path));
        let absolute_path = absolute_path.canonicalize().unwrap_or(absolute_path);
        let absolute_path_str = absolute_path.to_string_lossy().to_string();

        let actual_path = self.find_file(&absolute_path_str).unwrap_or(absolute_path_str);

        if !std::path::Path::new(&actual_path).exists() {
            return Err(format!("文件不存在: {}", actual_path).into());
        }

        let mut processed_files = self.processed_files.lock().unwrap();
        if *processed_files.get(&actual_path).unwrap_or(&false) {
            drop(processed_files);
            let hash = self.calculate_file_hash(&actual_path)?;
            let dir = std::path::Path::new(&actual_path).parent().unwrap().to_string_lossy().to_string();
            let filename = std::path::Path::new(&actual_path).file_name().unwrap().to_string_lossy().to_string();
            let clean_filename = self.remove_hash_from_filename(&filename);
            let hashed_filename = self.add_hash_to_filename(&clean_filename, &hash);
            let hashed_path = std::path::Path::new(&dir).join(&hashed_filename).to_string_lossy().to_string();

            return Ok(FileInfo {
                original_path: actual_path,
                hashed_path,
                hash,
                renamed: true,
            });
        }
        processed_files.insert(actual_path.clone(), true);
        drop(processed_files);

        if actual_path.to_lowercase().ends_with(".css") {
            self.process_component_css(&actual_path)
        } else {
            self.rename_file_with_hash(&actual_path)
        }
    }

    fn process_component_css(&self, css_path: &str) -> Result<FileInfo, Box<dyn std::error::Error>> {
        let css_dir = std::path::Path::new(css_path).parent().unwrap().to_string_lossy().to_string();
        let filename = std::path::Path::new(css_path).file_name().unwrap().to_string_lossy().to_string();
        let clean_filename = self.remove_hash_from_filename(&filename);

        let original_css_path = std::path::Path::new(&css_dir).join(&clean_filename);
        let original_css_path = if original_css_path.exists() {
            original_css_path.to_string_lossy().to_string()
        } else {
            css_path.to_string()
        };

        if self.debug_mode {
            println!("    📝 处理CSS: {}", clean_filename);
        }

        let images = self.collect_images_from_css(&original_css_path)?;
        let mut image_map = HashMap::new();

        if !images.is_empty() {
            println!("    📸 处理 {} 个图片引用", images.len());

            for image in images {
                let mut processed_files = self.processed_files.lock().unwrap();
                if *processed_files.get(&image.absolute_path).unwrap_or(&false) {
                    drop(processed_files);
                    let hash = self.calculate_file_hash(&image.absolute_path)?;
                    let old_image_filename = std::path::Path::new(&image.absolute_path).file_name().unwrap().to_string_lossy().to_string();
                    let clean_image_filename = self.remove_hash_from_filename(&old_image_filename);
                    let new_image_filename = self.add_hash_to_filename(&clean_image_filename, &hash);
                    image_map.insert(image.original_path, new_image_filename);
                    continue;
                }
                processed_files.insert(image.absolute_path.clone(), true);
                drop(processed_files);

                match self.rename_file_with_hash(&image.absolute_path) {
                    Ok(info) => {
                        let new_image_filename = std::path::Path::new(&info.hashed_path).file_name().unwrap().to_string_lossy().to_string();
                        image_map.insert(image.original_path, new_image_filename);

                        let rel_path = pathdiff::diff_paths(&image.absolute_path, &self.config.root_dir).unwrap_or_else(|| std::path::Path::new(&image.absolute_path).to_path_buf());
                        let mut version_map = self.version_map.lock().unwrap();
                        version_map.insert(rel_path.to_string_lossy().to_string(), info.hash);
                    },
                    Err(e) => {
                        println!("      ⚠️  失败: {} ({})", std::path::Path::new(&image.absolute_path).file_name().unwrap().to_string_lossy(), e);
                    }
                }
            }
        }

        let original_hash = self.calculate_file_hash(&original_css_path)?;
        let hashed_css_filename = self.add_hash_to_filename(&clean_filename, &original_hash);
        let hashed_css_path = std::path::Path::new(&css_dir).join(&hashed_css_filename);
        let hashed_css_path_str = hashed_css_path.to_string_lossy().to_string();

        std::fs::copy(&original_css_path, &hashed_css_path_str)?;

        if !image_map.is_empty() {
            self.update_css_image_references(&hashed_css_path_str, &image_map)?;

            let new_hash = self.calculate_file_hash(&hashed_css_path_str)?;
            if new_hash != original_hash {
                let final_css_filename = self.add_hash_to_filename(&clean_filename, &new_hash);
                let final_css_path = std::path::Path::new(&css_dir).join(&final_css_filename);
                let final_css_path_str = final_css_path.to_string_lossy().to_string();
                
                std::fs::rename(&hashed_css_path_str, &final_css_path_str)?;
            }
        }

        let css_ext = std::path::Path::new(&clean_filename).extension().unwrap_or_default().to_string_lossy().to_string();
        let css_basename = std::path::Path::new(&clean_filename).file_stem().unwrap().to_string_lossy().to_string();
        self.find_and_delete_old_hash_files(&css_dir, &css_basename, &css_ext, &original_hash)?;

        let rel_path = pathdiff::diff_paths(&original_css_path, &self.config.root_dir).unwrap_or_else(|| std::path::Path::new(&original_css_path).to_path_buf());
        let mut version_map = self.version_map.lock().unwrap();
        version_map.insert(rel_path.to_string_lossy().to_string(), original_hash.clone());

        Ok(FileInfo {
            original_path: original_css_path,
            hashed_path: hashed_css_path_str,
            hash: original_hash,
            renamed: true,
        })
    }

    fn update_html_references(&self, html_path: &str, resources: &HashMap<String, HashMap<String, String>>) -> Result<(), Box<dyn std::error::Error>> {
        let mut content = std::fs::read_to_string(html_path)?;
        let mut updated = false;

        if let Some(css_map) = resources.get("css") {
            for (original_rel_path, new_hashed_path) in css_map {
                let escaped_path = regex::escape(original_rel_path);
                let escaped_path = escaped_path.replace("/", r"[/\\]");
                
                let pattern = format!(r#"(<link[^>]*href\s*=\s*['"])({})(['"][^>]*>)"#, escaped_path);
                let re = Regex::new(&pattern)?;

                content = re.replace_all(&content, |caps: &regex::Captures| {
                    let prefix = &caps[1];
                    let old_path = &caps[2];
                    let suffix = &caps[3];

                    let old_dir = std::path::Path::new(old_path).parent().unwrap_or(std::path::Path::new("")).to_string_lossy().to_string();
                    let new_filename = std::path::Path::new(new_hashed_path).file_name().unwrap().to_string_lossy().to_string();

                    let mut new_path = if !old_dir.is_empty() && old_dir != "." && old_dir != "/" {
                        format!("{}/{}", old_dir, new_filename)
                    } else {
                        new_filename
                    };

                    if old_path.starts_with("../") || old_path.starts_with("..\\") {
                        if !new_path.starts_with("../") && !new_path.starts_with("..\\") {
                            new_path = format!("../{}", new_path);
                        }
                    } else if old_path.starts_with("./") || old_path.starts_with(".\\") {
                        if !new_path.starts_with("./") && !new_path.starts_with(".\\") {
                            new_path = format!("./{}", new_path);
                        }
                    }

                    if !self.config.cdn_domain.is_empty() && !new_path.starts_with("http") {
                        let clean_new_path = new_path.strip_prefix("./").unwrap_or(&new_path).strip_prefix("../").unwrap_or(&new_path);
                        new_path = format!("{}/{}", self.config.cdn_domain, clean_new_path);
                    }

                    let result = format!("{}{}{}", prefix, new_path, suffix);

                    if &caps[0] != &result {
                        updated = true;
                        println!("  ✅ CSS: {} -> {}", std::path::Path::new(old_path).file_name().unwrap().to_string_lossy(), std::path::Path::new(&new_path).file_name().unwrap().to_string_lossy());
                    }
                    result
                }).to_string();
            }
        }

        if let Some(js_map) = resources.get("js") {
            for (original_rel_path, new_hashed_path) in js_map {
                let escaped_path = regex::escape(original_rel_path);
                let escaped_path = escaped_path.replace("/", r"[/\\]");
                
                let pattern = format!(r#"(<script[^>]*src\s*=\s*['"])({})(['"][^>]*>)"#, escaped_path);
                let re = Regex::new(&pattern)?;

                content = re.replace_all(&content, |caps: &regex::Captures| {
                    let prefix = &caps[1];
                    let old_path = &caps[2];
                    let suffix = &caps[3];

                    let old_dir = std::path::Path::new(old_path).parent().unwrap_or(std::path::Path::new("")).to_string_lossy().to_string();
                    let new_filename = std::path::Path::new(new_hashed_path).file_name().unwrap().to_string_lossy().to_string();

                    let mut new_path = if !old_dir.is_empty() && old_dir != "." && old_dir != "/" {
                        format!("{}/{}", old_dir, new_filename)
                    } else {
                        new_filename
                    };

                    if old_path.starts_with("../") || old_path.starts_with("..\\") {
                        if !new_path.starts_with("../") && !new_path.starts_with("..\\") {
                            new_path = format!("../{}", new_path);
                        }
                    } else if old_path.starts_with("./") || old_path.starts_with(".\\") {
                        if !new_path.starts_with("./") && !new_path.starts_with(".\\") {
                            new_path = format!("./{}", new_path);
                        }
                    }

                    if !self.config.cdn_domain.is_empty() && !new_path.starts_with("http") {
                        let clean_new_path = new_path.strip_prefix("./").unwrap_or(&new_path).strip_prefix("../").unwrap_or(&new_path);
                        new_path = format!("{}/{}", self.config.cdn_domain, clean_new_path);
                    }

                    let result = format!("{}{}{}", prefix, new_path, suffix);

                    if &caps[0] != &result {
                        updated = true;
                        println!("  ✅ JS: {} -> {}", std::path::Path::new(old_path).file_name().unwrap().to_string_lossy(), std::path::Path::new(&new_path).file_name().unwrap().to_string_lossy());
                    }
                    result
                }).to_string();
            }
        }

        if updated {
            std::fs::write(html_path, content)?;
            println!("\n✅ HTML文件已更新");
        } else {
            println!("\n⚠️  没有内容需要更新");
        }

        Ok(())
    }

    fn process_html_file(&self, html_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        println!("{}", "=".repeat(60));
        println!("📄 处理: {}", html_path);
        println!("{}", "=".repeat(60));

        if !std::path::Path::new(html_path).exists() {
            return Err(format!("文件不存在: {}", html_path).into());
        }

        let html_dir = std::path::Path::new(html_path).parent().unwrap().to_string_lossy().to_string();
        let html_basename = std::path::Path::new(html_path).file_stem().unwrap().to_string_lossy().to_string();

        let mut resources = HashMap::new();
        resources.insert("css".to_string(), HashMap::new());
        resources.insert("js".to_string(), HashMap::new());

        println!("\n📦 处理主 JavaScript 文件...");
        let js_paths = [
            std::path::Path::new(&html_dir).join(format!("{}.js", html_basename)),
            std::path::Path::new(&html_dir).join("js").join(format!("{}.js", html_basename)),
            std::path::Path::new(&html_dir).join("scripts").join("js").join(format!("{}.js", html_basename)),
        ];

        let mut main_js_found = false;
        for js_path in &js_paths {
            let js_path_str = js_path.to_string_lossy().to_string();
            if let Some(actual_js_path) = self.find_file(&js_path_str) {
                if let Ok(info) = self.rename_file_with_hash(&actual_js_path) {
                    let rel_path = pathdiff::diff_paths(&actual_js_path, &html_dir).unwrap_or_else(|| std::path::Path::new(&actual_js_path).to_path_buf());
                    let rel_path_str = rel_path.to_string_lossy().replace('\\', "/");

                    let hashed_rel_path = pathdiff::diff_paths(&info.hashed_path, &html_dir).unwrap_or_else(|| std::path::Path::new(&info.hashed_path).to_path_buf());
                    let hashed_rel_path_str = hashed_rel_path.to_string_lossy().replace('\\', "/");

                    let normalized_key = rel_path_str.strip_prefix("./").unwrap_or(&rel_path_str).to_string();
                    if let Some(js_map) = resources.get_mut("js") {
                        js_map.insert(normalized_key, hashed_rel_path_str);
                    }

                    main_js_found = true;
                    break;
                }
            }
        }

        if !main_js_found {
            println!("  ℹ️  未找到主JS文件");
        }

        println!("\n🎨 处理主 CSS 文件...");
        let css_paths = [
            std::path::Path::new(&html_dir).join(format!("{}.css", html_basename)),
            std::path::Path::new(&html_dir).join("css").join(format!("{}.css", html_basename)),
        ];

        let mut main_css_found = false;
        for css_path in &css_paths {
            let css_path_str = css_path.to_string_lossy().to_string();
            if let Some(actual_css_path) = self.find_file(&css_path_str) {
                if let Ok(info) = self.process_component_css(&actual_css_path) {
                    let rel_path = pathdiff::diff_paths(&actual_css_path, &html_dir).unwrap_or_else(|| std::path::Path::new(&actual_css_path).to_path_buf());
                    let rel_path_str = rel_path.to_string_lossy().replace('\\', "/");

                    let hashed_rel_path = pathdiff::diff_paths(&info.hashed_path, &html_dir).unwrap_or_else(|| std::path::Path::new(&info.hashed_path).to_path_buf());
                    let hashed_rel_path_str = hashed_rel_path.to_string_lossy().replace('\\', "/");

                    let normalized_key = rel_path_str.strip_prefix("./").unwrap_or(&rel_path_str).to_string();
                    if let Some(css_map) = resources.get_mut("css") {
                        css_map.insert(normalized_key, hashed_rel_path_str);
                    }

                    main_css_found = true;
                    break;
                }
            }
        }

        if !main_css_found {
            println!("  ℹ️  未找到主CSS文件");
        }

        println!("\n🔍 扫描组件资源...");
        let html_resources = self.collect_resources_from_html(html_path)?;
        println!("  找到 {} 个组件CSS, {} 个组件JS", 
                 html_resources.get("css").map(|v| v.len()).unwrap_or(0),
                 html_resources.get("js").map(|v| v.len()).unwrap_or(0));

        if let Some(js_paths) = html_resources.get("js") {
            println!("\n🔧 处理组件 JavaScript 文件...");
            for js_rel_path in js_paths {
                let normalized_key = js_rel_path.strip_prefix("./").unwrap_or(js_rel_path).replace('\\', "/").to_string();
                
                if let Some(js_map) = resources.get("js") {
                    if js_map.contains_key(&normalized_key) {
                        continue;
                    }
                }

                match self.process_component_resource(&html_dir, js_rel_path) {
                    Ok(info) => {
                        let hashed_rel_path = pathdiff::diff_paths(&info.hashed_path, &html_dir).unwrap_or_else(|| std::path::Path::new(&info.hashed_path).to_path_buf());
                        let hashed_rel_path_str = hashed_rel_path.to_string_lossy().replace('\\', "/");

                        if let Some(js_map) = resources.get_mut("js") {
                            js_map.insert(normalized_key, hashed_rel_path_str);
                        }
                    },
                    Err(e) => {
                        println!("  ❌ 失败: {} ({})", js_rel_path, e);
                    }
                }
            }
        }

        if let Some(css_paths) = html_resources.get("css") {
            println!("\n🔧 处理组件 CSS 文件...");
            for css_rel_path in css_paths {
                let normalized_key = css_rel_path.strip_prefix("./").unwrap_or(css_rel_path).replace('\\', "/").to_string();
                
                if let Some(css_map) = resources.get("css") {
                    if css_map.contains_key(&normalized_key) {
                        continue;
                    }
                }

                match self.process_component_resource(&html_dir, css_rel_path) {
                    Ok(info) => {
                        let hashed_rel_path = pathdiff::diff_paths(&info.hashed_path, &html_dir).unwrap_or_else(|| std::path::Path::new(&info.hashed_path).to_path_buf());
                        let hashed_rel_path_str = hashed_rel_path.to_string_lossy().replace('\\', "/");

                        if let Some(css_map) = resources.get_mut("css") {
                            css_map.insert(normalized_key, hashed_rel_path_str);
                        }
                    },
                    Err(e) => {
                        println!("  ❌ 失败: {} ({})", css_rel_path, e);
                    }
                }
            }
        }

        println!("\n🔄 更新HTML中的资源引用...");
        println!("  📋 CSS: {} 项, JS: {} 项", 
                 resources.get("css").map(|m| m.len()).unwrap_or(0),
                 resources.get("js").map(|m| m.len()).unwrap_or(0));

        self.update_html_references(html_path, &resources)?;

        println!("\n✨ 处理完成!");
        Ok(())
    }

    fn process_multiple_html_files(&self, html_paths: Vec<String>) {
        println!("🚀 开始批量处理HTML文件...\n");

        for html_path in html_paths {
            let absolute_path = std::path::Path::new(&self.config.root_dir).join(&html_path).to_string_lossy().to_string();
            if let Err(e) = self.process_html_file(&absolute_path) {
                println!("❌ 处理失败 {}: {}", html_path, e);
            }
        }

        self.save_version_map();
        println!("\n{}", "=".repeat(60));
        println!("🎉 全部处理完成！");
        println!("{}", "=".repeat(60));
    }

    fn save_version_map(&self) {
        let version_map = self.version_map.lock().unwrap();
        if let Ok(json_data) = serde_json::to_string_pretty(&*version_map) {
            if std::fs::write(".version-map.json", json_data).is_ok() {
                println!("💾 版本映射已保存");
            }
        }
    }

    fn find_all_html_files(&self) -> Vec<String> {
        let mut html_files = Vec::new();
        
        if let Ok(entries) = walkdir::WalkDir::new(&self.config.root_dir)
            .into_iter()
            .filter_entry(|entry| {
                let name = entry.file_name().to_string_lossy();
                !self.config.exclude_dirs.contains(&name.to_string())
            })
            .collect::<Result<Vec<_>, _>>()
        {
            for entry in entries {
                if entry.file_type().is_file() && entry.path().extension().map(|e| e == "html").unwrap_or(false) {
                    if let Ok(rel_path) = entry.path().strip_prefix(&self.config.root_dir) {
                        html_files.push(rel_path.to_string_lossy().to_string());
                    }
                }
            }
        }

        html_files
    }
}

#[derive(Parser)]
#[clap(name = "version-manager", about = "HTML版本管理工具")]
struct Args {
    /// 配置文件路径
    #[clap(short = 'c', long, default_value = "version.config.json")]
    config: String,
    
    /// 单个HTML文件路径（命令行指定，优先级高于配置文件）
    #[clap(short, long)]
    file: Option<String>,
    
    /// 扫描所有HTML文件
    #[clap(long)]
    all: bool,
    
    /// CDN域名 (短参数改为 -d 避免与 -c 冲突)
    #[clap(short = 'd', long)]
    cdn: Option<String>,
    
    /// 调试模式（显示详细日志）
    #[clap(long)]
    debug: bool,
}

fn load_config(config_path: &str) -> Result<Config, Box<dyn std::error::Error>> {
    let data = std::fs::read_to_string(config_path)?;
    let mut config: Config = serde_json::from_str(&data)?;

    if config.root_dir.is_empty() {
        config.root_dir = ".".to_string();
    }
    
    if config.hash_length == 0 {
        config.hash_length = 8;
    }
    
    if config.exclude_dirs.is_empty() {
        config.exclude_dirs = vec!["node_modules".to_string(), ".git".to_string(), "dist".to_string(), "build".to_string()];
    }

    let is_home = env::var("IS_HOME").unwrap_or_default();
    println!("📍 环境变量 IS_HOME={}", is_home);

    if !config.home_html_file.is_empty() || !config.company_html_file.is_empty() {
        if is_home == "1" {
            if !config.home_html_file.is_empty() {
                config.single_html_file = config.home_html_file.clone();
                println!("🏠 使用家里电脑路径: {}", config.single_html_file);
            }
        } else {
            if !config.company_html_file.is_empty() {
                config.single_html_file = config.company_html_file.clone();
                println!("🏢 使用公司电脑路径: {}", config.single_html_file);
            }
        }
    }

    Ok(config)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let config = match load_config(&args.config) {
        Ok(config) => config,
        Err(_) => Config {
            root_dir: ".".to_string(),
            cdn_domain: String::new(),
            hash_length: 8,
            single_html_file: String::new(),
            html_files: Vec::new(),
            exclude_dirs: vec!["node_modules".to_string(), ".git".to_string(), "dist".to_string(), "build".to_string()],
            home_html_file: String::new(),
            company_html_file: String::new(),
            include_components: Vec::new(),
        },
    };

    let mut config = config;
    if let Some(cdn_domain) = args.cdn {
        config.cdn_domain = cdn_domain;
    }

    let vm = VersionManager::new(config, args.debug);

    if !vm.config.include_components.is_empty() {
        println!("📋 指定处理组件: {:?}", vm.config.include_components);
    } else {
        println!("📋 处理所有组件");
    }

    let target_html_file = args.file.or_else(|| {
        if !vm.config.single_html_file.is_empty() {
            println!("📋 使用配置文件中的HTML文件");
            Some(vm.config.single_html_file.clone())
        } else {
            None
        }
    });

    if let Some(html_file) = target_html_file {
        vm.process_html_file(&html_file)?;
        vm.save_version_map();
        return Ok(());
    }

    if args.all {
        let html_files = vm.find_all_html_files();
        println!("📋 找到 {} 个HTML文件\n", html_files.len());
        if !html_files.is_empty() {
            vm.process_multiple_html_files(html_files);
        } else {
            println!("❌ 未找到HTML文件");
        }
        return Ok(());
    }

    if !vm.config.html_files.is_empty() {
        vm.process_multiple_html_files(vm.config.html_files.clone());
    } else {
        println!("⚠️  未指定要处理的HTML文件");
        println!("使用 --file 指定文件, --all 扫描所有, 或在配置文件中指定");
    }

    Ok(())
}