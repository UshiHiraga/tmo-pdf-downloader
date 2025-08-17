mod models;
mod utils;
use models::CliError;
use models::Manga;
use printpdf::PdfSaveOptions;
use utils::io::get_manga_cache;
use utils::io::set_manga_cache;
use utils::network::fetch;
use utils::parser::html_paginated_json;
use utils::parser::manga_html_to_json;

use clap::Parser;
use dirs::cache_dir;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use url::Url;
use std::time::Instant;

fn px_to_mm(px: f32) -> f32 {
    // mm = ( pixels * 25.4 ) / DPI
    return px * 25.4 / 300.0;
}

fn get_cache_path() -> PathBuf {
    let cache_path = cache_dir().expect("No path to folder");
    cache_path.join("tmo-pdf-downloader")
}

fn extract_id_from_url(input: &str) -> Result<u32, CliError> {
    let url = match Url::parse(input) {
        Ok(u) => u,
        Err(e) => return Err(CliError::InvalidUrl(e.to_string())),
    };

    match url.host_str() {
        Some(host) => {
            if host != "zonatmo.com" {
                return Err(CliError::InvalidMangaUrl);
            }
        }
        None => return Err(CliError::InvalidMangaUrl),
    }

    let segments: Vec<&str> = match url.path_segments() {
        Some(segments) => segments.collect(),
        None => return Err(CliError::MissingId),
    };

    if segments.len() < 2 {
        return Err(CliError::MissingId);
    }

    if segments[0] != "library" {
        return Err(CliError::InvalidMangaUrl);
    }

    let manga_id = match segments[2].parse::<u32>() {
        Ok(res) => res,
        Err(_) => return Err(CliError::InvalidMangaUrl),
    };

    return Ok(manga_id);
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, group = "source")]
    url: Option<String>,

    #[arg(short, long, group = "source")]
    index: Option<u32>,

    #[arg(short, long)]
    chapter_index: Option<u32>, // chapter index

    #[arg(short, long)]
    no_cache: bool,
}

fn main() {
    let program_time = Instant::now();
    // 1
    let args = Args::parse();
    let url = args.url.clone();
    let manga_index = match (args.url, args.index) {
        (None, None) => unreachable!(),
        (Some(_), Some(_)) => unreachable!(),
        (Some(url), None) => extract_id_from_url(&url).expect("The url did not contain the index"),
        (None, Some(index)) => index,
    };

    println!("{:?}", manga_index);
    println!("{:?}", args.no_cache);

    // 2
    // Check location on cache
    let cache_path = get_cache_path();
    // is manga cached?
    let manga: Manga = match get_manga_cache(&cache_path, &manga_index.to_string()) {
        Ok(res) => {
            println!("Got from cache");
            res
        }
        Err(_) => {
            println!("Caché no encontrada. Haciendo fetch de los datos.");
            let response = fetch(&url.unwrap().to_string()).expect("Error on fecth");
            let html_file = response.text().expect("Incorrect body");
            let manga =
                manga_html_to_json(manga_index.to_string(), html_file).expect("error on parsing");
            let _ = set_manga_cache(&cache_path, &manga_index.to_string(), &manga);
            manga
        }
    };

    // 3  https://zonatmo.com/view_uploads/1650650
    // as no chapter still, we'll take first
    // this can fail if one_shot
    let chap_index: usize = match args.chapter_index {
        Some(res) => res as usize,
        None => 0,
    };

    println!("chapter selected had the index {}", chap_index);

    let url_chap_view = &manga.chapters[chap_index].views[0].link;
    let chap_name = &manga.chapters[chap_index].name;
    let ss = fetch(&url_chap_view).unwrap().text().unwrap();
    let urss = html_paginated_json(ss).unwrap(); //implemet 
    println!("we got the urls for the chapter");

    // Itera sobre las url y trata de descargar las imagenes.
    let folder_chapter = cache_path.join(manga_index.to_string()).join(chap_name);
    let mut images_path: Vec<PathBuf> = Vec::new();
    let _ = fs::create_dir_all(folder_chapter.clone());
    println!("we'll start fetching images");
    let fetching_time = Instant::now();
    for (i, image_url) in urss.iter().enumerate() {
        let response = fetch(&image_url.as_str().to_string()).expect("error no fetch");
        let bytes = response.bytes().expect("error on parsing images bytes");
        let path = folder_chapter.join(format!("{}.webp", i));
        fs::write(&path, bytes).expect("no write");
        println!("image {} saved on disk.", i);
        images_path.push(path);
    }
    println!("all the fetch ellpased {} seconds", fetching_time.elapsed().as_secs());
    // images_path = fs::read_dir(folder_chapter).unwrap().map(|x| x.unwrap().path()).collect();

    // 4
    // crea el pdf
    println!("we got all the images, now the pdf");
    let mut ub_final = File::create("test.pdf").expect("create file");
    create_pdf(&mut ub_final, &chap_name, &images_path);
    println!("finish in {} seconds", program_time.elapsed().as_secs());
}

use printpdf::{Mm, Op, PdfDocument, PdfPage, PdfWarnMsg, Pt, RawImage, XObjectTransform};
use std::fs::File;
fn create_pdf(file: &mut File, name: &str, paths: &Vec<PathBuf>) {
    let mut warnings: Vec<PdfWarnMsg> = Vec::new();
    let mut document = PdfDocument::new(name);

    let mut pages: Vec<PdfPage> = Vec::new();
    for image_path in paths {
        let letter_dimension = (8.5 * 25.4, 11.0 * 25.4);

        let image_bytes = fs::read(image_path).expect("image not found");
        let image = RawImage::decode_from_bytes(&image_bytes, &mut warnings).expect("decode");
        let image_ratio = (image.height / image.width) as f32;

        let image_xobject_ref = document.add_image(&image);
        let page_content = vec![Op::UseXobject {
            id: image_xobject_ref,
            transform: XObjectTransform {
                translate_x: Some(Pt(0.0)),
                translate_y: Some(Pt(0.0)),
                scale_x: Some(letter_dimension.0 / px_to_mm(image.width as f32)),
                scale_y: Some(image_ratio * letter_dimension.0 / px_to_mm(image.height as f32)),
                dpi: None,
                rotate: None,
            }, // transform to fit in a letter page.
        }];

        // let width = Mm(px_to_mm(image.width as f32));
        // let heigth = Mm(px_to_mm(image.height as f32));
        let page = PdfPage::new(
            Mm(letter_dimension.0),
            Mm(image_ratio * letter_dimension.0),
            page_content,
        ); // letter page;
        pages.push(page);
    }

    let pdf_bytes = document
        .with_pages(pages)
        .save(&PdfSaveOptions::default(), &mut warnings);

    println!("we'll start to write");
    file.write(&pdf_bytes).expect("good write");
}
