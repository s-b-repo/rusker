// Cargo.toml dependencies
// [dependencies]
// reqwest = { version = "0.11", features = ["blocking"] }
// scraper = "0.14"
// csv = "1.1"
// calamine = "0.21"
 
use reqwest::blocking::Client;
use scraper::{Html, Selector};
use std::error::Error;
use std::fs::File;
use std::io::Write;
use csv::Writer;
use calamine::{Xlsx, XlsxWriter, open_workbook, DataType, WriterManager};
 
fn main() -> Result<(), Box<dyn Error>> {
    let dork = "site:example.com intitle:index.of";
    let search_results = scrape_results(dork)?;
 
    save_as_csv(&search_results, "results.csv")?;
    save_as_spreadsheet(&search_results, "results.xlsx")?;
 
    Ok(())
}
 
fn scrape_results(dork: &str) -> Result<Vec<(String, String)>, Box<dyn Error>> {
    let client = Client::new();
    let url = format!("https://www.google.com/search?q={}", dork);
    let response = client.get(&url).send()?.text()?;
 
    let document = Html::parse_document(&response);
    let selector = Selector::parse("h3 > a").unwrap(); // Adjust based on actual HTML structure
    let mut results = Vec::new();
 
    for element in document.select(&selector) {
        let title = element.inner_html();
        let link = element.value().attr("href").unwrap_or("").to_string();
        results.push((title, link));
    }
 
    Ok(results)
}
 
fn save_as_csv(results: &[(String, String)], filename: &str) -> Result<(), Box<dyn Error>> {
    let mut wtr = Writer::from_path(filename)?;
    wtr.write_record(&["Title", "Link"])?;
 
    for (title, link) in results {
        wtr.write_record(&[title, link])?;
    }
 
    wtr.flush()?;
    Ok(())
}
 
fn save_as_spreadsheet(results: &[(String, String)], filename: &str) -> Result<(), Box<dyn Error>> {
    let mut workbook = XlsxWriter::new(File::create(filename)?)?;
    let mut sheet = workbook.add_worksheet(Some("Results"))?;
 
    sheet.write_string(0, 0, "Title")?;
    sheet.write_string(0, 1, "Link")?;
 
    for (i, (title, link)) in results.iter().enumerate() {
        sheet.write_string((i + 1) as u32, 0, title)?;
        sheet.write_string((i + 1) as u32, 1, link)?;
    }
 
    workbook.close()?;
    Ok(())
}
 
