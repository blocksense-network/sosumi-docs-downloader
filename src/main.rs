use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

use anyhow::{Context, Result};
use clap::Parser;
use futures::future::join_all;
use reqwest::Client;
use url::Url;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Documentation paths to download (e.g., endpointsecurity swift/array)
    #[arg(required = true)]
    doc_types: Vec<String>,

    /// Output directory (default: current directory)
    #[arg(short, long, default_value = ".")]
    outdir: String,

    /// Maximum crawl depth (default: unlimited)
    #[arg(short = 'd', long, default_value = "0")]
    max_depth: usize,
}

struct Downloader {
    client: Client,
    doc_type: String,
    output_dir: String,
    target_dir: String,
    base_url: String,
}

impl Downloader {
    fn new(doc_type: String, output_dir: String) -> Self {
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .build()
            .expect("Failed to create HTTP client");

        let target_dir = format!("{}/{}", output_dir, doc_type);
        let base_url = "https://sosumi.ai".to_string();

        Self {
            client,
            doc_type,
            output_dir,
            target_dir,
            base_url,
        }
    }

    async fn run(&self, max_depth: Option<usize>) -> Result<()> {

        // Create target directory
        fs::create_dir_all(&self.target_dir)?;

        // Clear existing contents
        if Path::new(&self.target_dir).exists() {
            println!("🗑️  Removing existing {} directory contents...", self.doc_type);
            fs::remove_dir_all(&self.target_dir)?;
            fs::create_dir_all(&self.target_dir)?;
        }

        println!("📥 Starting download process...");
        println!("🔍 Using concurrent Rust scraper to download documentation...");

        let start_url = format!("{}/documentation/{}", self.base_url, self.doc_type);
        let visited = Arc::new(Mutex::new(HashSet::new()));
        let to_visit = vec![start_url];

        // Start crawling
        self.crawl(to_visit, visited, 0, max_depth).await?;

        // Count downloaded files
        let file_count = self.count_markdown_files()?;
        println!("📊 Downloaded {} markdown files", file_count);

        println!("✅ Crawl completed!");

        Ok(())
    }

    async fn crawl(
        &self,
        to_visit: Vec<String>,
        visited: Arc<Mutex<HashSet<String>>>,
        depth: usize,
        max_depth: Option<usize>,
    ) -> Result<()> {
        if let Some(max) = max_depth {
            if depth >= max || to_visit.is_empty() {
                return Ok(());
            }
        } else if to_visit.is_empty() {
            return Ok(());
        }

        println!("\n📊 Depth {}: {} URLs to process", depth + 1, to_visit.len());

        // Download all pages in this depth level concurrently
        let mut download_tasks = Vec::new();
        {
            let mut visited_guard = visited.lock().await;
            for url in &to_visit {
                if !visited_guard.contains(url) {
                    visited_guard.insert(url.clone());
                    let task = self.download_page(url.clone());
                    download_tasks.push(task);
                }
            }
        }

        // Wait for all downloads to complete
        let results = join_all(download_tasks).await;

            // Process results and extract new links
        let mut next_visit = Vec::new();
        for result in results {
            match result {
                Ok(Some((url, content))) => {
                    // Extract links if we haven't reached max depth
                    let should_extract = match max_depth {
                        Some(max) => depth < max - 1,
                        None => true, // unlimited depth
                    };

                    if should_extract {
                        // Extract links from this page
                        let links = self.extract_markdown_links(&content, &url)?;

                        // Filter to our documentation section
                        let doc_links: Vec<String> = links
                            .into_iter()
                            .filter(|link| link.contains(&format!("/documentation/{}", self.doc_type)))
                            .collect();

                        if !doc_links.is_empty() {
                            println!("    🔗 Found {} new links from {}", doc_links.len(), url);
                        }

                        next_visit.extend(doc_links);
                    }

                    // Convert links in the downloaded content
                    let filepath = self.url_to_filepath(&url)?;
                    let converted_content = self.convert_links_to_relative(&content)?;
                    if converted_content != content {
                        fs::write(&filepath, converted_content)?;
                    }
                }
                Ok(None) => {} // Skipped page
                Err(e) => {
                    println!("    ❌ Download error: {}", e);
                }
            }
        }

        // Remove duplicates and recurse
        let mut unique_next: Vec<String> = next_visit.into_iter().collect::<HashSet<_>>().into_iter().collect();
        {
            let visited_guard = visited.lock().await;
            unique_next.retain(|url| !visited_guard.contains(url));
        }

        if !unique_next.is_empty() {
            Box::pin(self.crawl(unique_next, visited, depth + 1, max_depth)).await?;
        }

        Ok(())
    }

    async fn download_page(&self, url: String) -> Result<Option<(String, String)>> {
        let response = self.client
            .get(&url)
            .send()
            .await
            .with_context(|| format!("Failed to request {}", url))?;

        let status = response.status();
        if !status.is_success() {
            return Err(anyhow::anyhow!("HTTP {} for {}", status, url));
        }

        let content = response.text().await?;

        // Skip if it looks like an error page
        if regex::Regex::new(r"(?i)404|Not Found|Error").unwrap().is_match(&content) && content.len() < 1000 {
            println!("    ⚠️  Skipping {} (error page)", url);
            return Ok(None);
        }

        let filepath = self.url_to_filepath(&url)?;
        fs::create_dir_all(Path::new(&filepath).parent().unwrap())?;
        fs::write(&filepath, &content)?;

        println!("    ✅ Downloaded {} → {}", url, filepath);

        Ok(Some((url, content)))
    }

    fn extract_markdown_links(&self, content: &str, base_url: &str) -> Result<Vec<String>> {
        let mut links = Vec::new();
        let mut i = 0;
        let chars: Vec<char> = content.chars().collect();

        while i < chars.len() {
            // Find opening [
            if chars[i] != '[' {
                i += 1;
                continue;
            }

            // Find closing ] for link text
            let text_start = i + 1;
            let mut text_end = text_start;
            while text_end < chars.len() && chars[text_end] != ']' {
                text_end += 1;
            }
            if text_end >= chars.len() {
                i += 1;
                continue;
            }

            // Check for opening ( after ]
            if text_end + 1 >= chars.len() || chars[text_end + 1] != '(' {
                i = text_end + 1;
                continue;
            }

            // Find closing ) for URL - account for nested parentheses
            let url_start = text_end + 2;
            let mut paren_count = 0;
            let mut url_end = url_start;

            while url_end < chars.len() {
                match chars[url_end] {
                    '(' => paren_count += 1,
                    ')' => {
                        if paren_count == 0 {
                            break; // Found the closing ) of the markdown link
                        }
                        paren_count -= 1;
                    }
                    _ => {}
                }
                url_end += 1;
            }

            if url_end >= chars.len() {
                i = text_end + 1;
                continue;
            }

            let link_url: String = chars[url_start..url_end].iter().collect();

            // Skip external links and anchors
            if link_url.starts_with("http") || link_url.starts_with('#') {
                i = url_end + 1;
                continue;
            }

            // Convert relative links to absolute
            let full_url = if link_url.starts_with('/') {
                format!("{}{}", self.base_url, link_url)
            } else {
                let base = Url::parse(base_url)?;
                base.join(&link_url)?.to_string()
            };

            links.push(full_url);
            i = url_end + 1;
        }

        Ok(links)
    }

    fn url_to_filepath(&self, url: &str) -> Result<String> {
        let parsed = Url::parse(url)?;
        let mut path = parsed.path().to_string();

        // Remove leading /documentation/DOC_TYPE
        let prefix = format!("/documentation/{}", self.doc_type);
        if path.starts_with(&prefix) {
            path = path[prefix.len()..].to_string();
        } else if path.starts_with(&format!("/{}", self.doc_type)) {
            path = path[self.doc_type.len() + 1..].to_string();
        } else if path == format!("/documentation/{}", self.doc_type) {
            path = String::new();
        }

        // Remove leading slash
        path = path.trim_start_matches('/').to_string();

        // If empty, use index
        if path.is_empty() {
            path = "index".to_string();
        }

        // Add .md extension if not present
        if !Path::new(&path).extension().is_some() {
            path.push_str(".md");
        }

        Ok(format!("{}/{}", self.target_dir, path))
    }

    fn convert_links_to_relative(&self, content: &str) -> Result<String> {
        // We need to convert links while preserving URLs with parentheses
        let mut result = String::new();
        let chars: Vec<char> = content.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            // Find opening [
            if chars[i] != '[' {
                result.push(chars[i]);
                i += 1;
                continue;
            }

            // Find closing ] for link text
            let text_start = i + 1;
            let mut text_end = text_start;
            while text_end < chars.len() && chars[text_end] != ']' {
                text_end += 1;
            }
            if text_end >= chars.len() {
                result.push(chars[i]);
                i += 1;
                continue;
            }

            let link_text: String = chars[text_start..text_end].iter().collect();

            // Check for opening ( after ]
            if text_end + 1 >= chars.len() || chars[text_end + 1] != '(' {
                result.push(chars[i]);
                i = text_end + 1;
                continue;
            }

            // Find closing ) for URL - account for nested parentheses
            let url_start = text_end + 2;
            let mut paren_count = 0;
            let mut url_end = url_start;

            while url_end < chars.len() {
                match chars[url_end] {
                    '(' => paren_count += 1,
                    ')' => {
                        if paren_count == 0 {
                            break; // Found the closing ) of the markdown link
                        }
                        paren_count -= 1;
                    }
                    _ => {}
                }
                url_end += 1;
            }

            if url_end >= chars.len() {
                result.push(chars[i]);
                i = text_end + 1;
                continue;
            }

            let link_url: String = chars[url_start..url_end].iter().collect();

            // Convert the link if it's an internal documentation link
            if link_url.starts_with(&format!("/documentation/{}", self.doc_type)) {
                // Remove the /documentation/doc_type prefix
                let mut relative_path = link_url[format!("/documentation/{}", self.doc_type).len()..].to_string();
                if relative_path.is_empty() {
                    relative_path = "/".to_string();
                } else if !relative_path.starts_with('/') {
                    relative_path = format!("/{}", relative_path);
                }

                // Convert to relative path
                let new_url = if relative_path == "/" {
                    "./index.md".to_string()
                } else {
                    // Remove leading slash and add .md extension if needed
                    let mut rel_path = relative_path.trim_start_matches('/').to_string();
                    if !Path::new(&rel_path).extension().is_some() {
                        rel_path.push_str(".md");
                    }
                    format!("./{}", rel_path)
                };

                result.push_str(&format!("[{}]({})", link_text, new_url));
            } else {
                // Keep the original link
                result.push_str(&format!("[{}]({})", link_text, link_url));
            }

            i = url_end + 1;
        }

        // Handle any remaining content
        while i < chars.len() {
            result.push(chars[i]);
            i += 1;
        }

        Ok(result)
    }

    fn count_markdown_files(&self) -> Result<usize> {
        let mut count = 0;
        for entry in walkdir::WalkDir::new(&self.target_dir) {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "md" || ext == "markdown" {
                        count += 1;
                    }
                }
            }
        }
        Ok(count)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Convert max_depth: 0 means unlimited, otherwise use the specified value
    let max_depth = if args.max_depth == 0 { None } else { Some(args.max_depth) };

    println!("🔄 Downloading {} documentation type(s) to {}", args.doc_types.len(), args.outdir);
    println!("📁 Output directory: {}", std::fs::canonicalize(&args.outdir)
         .unwrap_or_else(|_| std::path::PathBuf::from(&args.outdir)).display());

    for doc_type in &args.doc_types {
        println!("\n📚 Processing documentation: {}", doc_type);
        let downloader = Downloader::new(doc_type.clone(), args.outdir.clone());
        downloader.run(max_depth).await?;
    }

    Ok(())
}
