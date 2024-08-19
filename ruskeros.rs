// Cargo.toml dependencies
// [dependencies]
// reqwest = { version = "0.11", features = ["blocking"] }
// scraper = "0.14"
// csv = "1.1"
// calamine = "0.21"
// rand = "0.8"
// log = "0.4"
// env_logger = "0.10"
// clap = { version = "4.0", features = ["derive"] }
 
use reqwest::blocking::Client;
use reqwest::header::USER_AGENT;
use scraper::{Html, Selector};
use std::{error::Error, fs::File, io::{self, BufRead, Write}, thread, time::Duration};
use csv::Writer;
use calamine::{XlsxWriter, WriterManager};
use rand::seq::SliceRandom;
use rand::Rng;
use log::{info, error};
use clap::Parser;
 
/// Command-line arguments
#[derive(Parser)]
struct Cli {
    /// Number of requests to make per dork
    #[arg(short, long, default_value_t = 5)]
    requests: usize,
 
    /// Minimum delay between requests (in seconds)
    #[arg(short, long, default_value_t = 1)]
    min_delay: u64,
 
    /// Maximum delay between requests (in seconds)
    #[arg(short, long, default_value_t = 5)]
    max_delay: u64,
 
    /// Dork to search for (if dorks_file is not provided)
    #[arg(short, long)]
    dork: Option<String>,
 
    /// Path to a file containing multiple dorks (one per line)
    #[arg(short = 'f', long)]
    dorks_file: Option<String>,
 
    /// Maximum number of retries for failed requests
    #[arg(short, long, default_value_t = 3)]
    max_retries: usize,
}
 
fn main() -> Result<(), Box<dyn Error>> {
    // Initialize the logger
    env_logger::init();
 
    let args = Cli::parse();
 
    // Read dorks from file or use the provided single dork
    let dorks = match &args.dorks_file {
        Some(file_path) => {
            let file = File::open(file_path)?;
            io::BufReader::new(file).lines().collect::<Result<Vec<String>, _>>()?
        }
        None => vec![args.dork.expect("Either --dork or --dorks_file must be provided.")],
    };
 
    for dork in dorks {
        match scrape_results(&dork, args.requests, args.min_delay, args.max_delay, args.max_retries) {
            Ok(search_results) => {
                save_as_csv(&search_results, &dork)?;
                save_as_spreadsheet(&search_results, &dork)?;
            }
            Err(e) => {
                error!("Failed to scrape results for dork '{}': {}", dork, e);
            }
        }
    }
 
    Ok(())
}
 
fn scrape_results(dork: &str, num_requests: usize, min_delay: u64, max_delay: u64, max_retries: usize) -> Result<Vec<(String, String)>, Box<dyn Error>> {
    let client = Client::new();
    let user_agents = vec![
        "Mozilla/5.0 (compatible; Googlebot/2.1; +http://www.google.com/bot.html)",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36",
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Firefox/89.0",
        "Mozilla/5.0 (compatible; Bingbot/2.0; +http://www.bing.com/bingbot.htm)"
    ];
 
    let url = format!("https://www.google.com/search?q={}", dork);
    let mut results = Vec::new();
 
    for i in 0..num_requests {
        let user_agent = user_agents.choose(&mut rand::thread_rng()).unwrap(); // Randomly select a User-Agent
        info!("Scraping with User-Agent: {} for dork: {}", user_agent, dork);
 
        let mut retries = 0;
        let mut success = false;
 
        while retries < max_retries && !success {
            match client.get(&url)
                .header(USER_AGENT, user_agent)
                .send() {
                    Ok(response) => {
                        let response_text = response.text()?;
                        let document = Html::parse_document(&response_text);
                        let selector = Selector::parse("h3 > a").unwrap(); // Adjust based on actual HTML structure
 
                        for element in document.select(&selector) {
                            let title = element.inner_html();
                            let link = element.value().attr("href").unwrap_or("").to_string();
                            results.push((title, link));
                            info!("Scraped URL: {}", link);
                        }
                        success = true;
                    }
                    Err(e) => {
                        retries += 1;
                        error!("Failed to send request #{}: {}, retrying ({}/{})", i + 1, e, retries, max_retries);
                        if retries >= max_retries {
                            error!("Max retries reached for request #{}", i + 1);
                        }
                    }
                }
 
            if !success {
                let delay = rand::thread_rng().gen_range(min_delay..=max_delay);
                info!("Sleeping for {} seconds before retrying", delay);
                thread::sleep(Duration::from_secs(delay));
            }
        }
 
        let delay = rand::thread_rng().gen_range(min_delay..=max_delay);
        info!("Sleeping for {} seconds before the next request", delay);
        thread::sleep(Duration::from_secs(delay)); // Add a random delay between requests
    }
 
    Ok(results)
}
 
fn save_as_csv(results: &[(String, String)], dork: &str) -> Result<(), Box<dyn Error>> {
    // Create a valid filename from the dork
    let filename = format!("{}_results.csv", sanitize_filename::sanitize(dork));
    let mut wtr = Writer::from_path(&filename)?;
    wtr.write_record(&["Title", "Link"])?;
 
    for (title, link) in results {
        wtr.write_record(&[title, link])?;
    }
 
    wtr.flush()?;
    info!("Saved results to {}", filename);
    Ok(())
}
 
fn save_as_spreadsheet(results: &[(String, String)], dork: &str) -> Result<(), Box<dyn Error>> {
    // Create a valid filename from the dork
    let filename = format!("{}_results.xlsx", sanitize_filename::sanitize(dork));
    let mut workbook = XlsxWriter::new(File::create(&filename)?)?;
    let mut sheet = workbook.add_worksheet(Some("Results"))?;
 
    sheet.write_string(0, 0, "Title")?;
    sheet.write_string(0, 1, "Link")?;
 
    for (i, (title, link)) in results.iter().enumerate() {
        sheet.write_string((i + 1) as u32, 0, title)?;
        sheet.write_string((i + 1) as u32, 1, link)?;
    }
 
    workbook.close()?;
    info!("Saved results to {}", filename);
    Ok(())
}
 
