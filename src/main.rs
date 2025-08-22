mod errors;
mod models;
mod utils;
use models::pages::ChapterParser;
use models::serie::Manga;
use printpdf::PdfSaveOptions;

use utils::fetch;

use clap::Parser;
use clap::ValueEnum;
use clap::error as ClapError;

use dirs::cache_dir;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

use std::time::Instant;
use url::Url;

fn px_to_mm(px: f32) -> f32 {
    // mm = ( pixels * 25.4 ) / DPI
    return px * 25.4 / 300.0;
}

fn get_cache_path() -> PathBuf {
    let cache_path = cache_dir().expect("No path to folder");
    cache_path.join("tmo-pdf-downloader")
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    /// ID del manga o manwha a descargar
    #[arg(required = true, value_parser = parse_url )]
    id: (u32, Option<PartialManga>),

    // El grupo de argumentos para la selección de capítulos
    /// Número de capítulo a descargar
    #[arg(short, long, group = "selection")]
    chapter: Option<u32>,

    /// Rango de capítulos a descargar (ej: 10-15, 25-end)
    #[arg(short, long, group = "selection")]
    range: Option<String>,

    /// Descargar todos los capítulos disponibles
    #[arg(short, long, group = "selection")]
    all: bool,

    /// Descargar solo el último capítulo disponible (por defecto si no se especifica nada)
    #[arg(
        short,
        long,
        group = "selection",
        default_value_t = true,
        default_missing_value = "true"
    )]
    last: bool,

    /// Ruta de salida para los archivos descargados
    #[arg(short, long, value_name = "PATH")]
    output: Option<String>,

    /// Output file.
    #[arg(value_enum, long, default_value_t = FormatOutput::Pdf)]
    format: FormatOutput,

    /// Avoid reading or writing to cache.
    #[arg(long)]
    no_cache: bool,
}

#[derive(Clone, ValueEnum)]
enum FormatOutput {
    Pdf,
    Images,
}

#[derive(Clone)]
struct PartialManga {
    url: Url,
}

fn parse_url(s: &str) -> Result<(u32, Option<PartialManga>), ClapError::Error> {
    // Try to parse as a numeric ID.
    if let Ok(index) = s.parse::<u32>() {
        return Ok((index, None));
    };

    // If it's not a number, try to parse as a URL.
    let url = match Url::parse(s) {
        Ok(u) => u,
        Err(_) => {
            return Err(ClapError::Error::raw(
                ClapError::ErrorKind::InvalidValue,
                "The value must be a numeric ID or a valid URL.",
            ));
        }
    };

    // Validate the URL host
    if url.host_str() != Some("zonatmo.com") {
        return Err(ClapError::Error::raw(
            ClapError::ErrorKind::InvalidValue,
            "The URL is not from zonatmo.com.",
        ));
    };

    // Get the path segments
    let segments: Vec<&str> = match &url.path_segments() {
        Some(s) => s.clone().collect(),
        None => {
            return Err(ClapError::Error::raw(
                ClapError::ErrorKind::InvalidValue,
                "The TMO URL must have a path with segments.",
            ));
        }
    };

    // Validate the path structure and extract the ID
    if segments.len() < 4 || segments[0] != "library" {
        return Err(ClapError::Error::raw(
            ClapError::ErrorKind::InvalidValue,
            "Invalid URL format. It should be similar to https://zonatmo.com/library/manga/12345/name.",
        ));
    };

    let manga_id = match segments[2].parse::<u32>() {
        Ok(res) => res,
        Err(_) => {
            return Err(ClapError::Error::raw(
                ClapError::ErrorKind::InvalidValue,
                "The URL does not contain a valid numeric manga ID.",
            ));
        }
    };

    let partial = PartialManga { url };

    return Ok((manga_id, Some(partial)));
}

fn main() {
    let program_time = Instant::now();
    // 1
    let args = Args::parse();

    let cache_path = get_cache_path();

    if args.no_cache && args.id.1.is_none() {
        panic!("we cant get from cache and we cant fecth due to we have not the url.");
    }

    let manga: Manga = match Manga::from_cache(&cache_path, &args.id.0.to_string()) {
        Ok(r) => r,
        Err(_) => {
            // we can get from cache

            if args.id.1.is_none() {
                panic!("we cant get from cache and we cant fecth due to we have not the url.");
            }

            let url = args.id.1.unwrap().url;
            let index = args.id.0.to_string();

            println!("Caché no encontrada. Haciendo fetch de los datos.");
            let response = fetch(&url.to_string()).expect("Error on fecth");
            let html_file = response.text().expect("Incorrect body");
            let manga = Manga::from_html(url, html_file).expect("error on parsing");

            if !args.no_cache {
                let _ = manga.to_cache(&cache_path, &index.to_string());
            }

            manga
        }
    };

    // // as no chapter still, we'll take first
    // // this can fail if one_shot
    // let chap_index: usize = match args.chapter_index {
    //     Some(res) => res as usize,
    //     None => 0,
    // };

    let chap_index: usize = 0;
    println!("chapter selected had the index {}", chap_index);

    let url_chap_view = &manga.chapters[chap_index].views[0].link;
    let chap_name = &manga.chapters[chap_index].name;
    let ss = fetch(&url_chap_view).unwrap().text().unwrap();
    let _ = fs::write("test.html", &ss);
    let urss = ChapterParser::get_images(&ss).unwrap();
    println!("we got the urls for the chapter");

    // Itera sobre las url y trata de descargar las imagenes.
    let folder_chapter = cache_path.join(manga.index.to_string()).join(chap_name);
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
    println!(
        "all the fetch ellpased {} seconds",
        fetching_time.elapsed().as_secs()
    );

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
