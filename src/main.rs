use anyhow::Result;
use async_std;
use std::fs;
use std::path::Path;
use hyper;
use clap::Parser;
use octocrab::*;
use octocrab::models::*;
use walkdir::{WalkDir, DirEntry};

mod strings;
mod source;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {

    // Path to the project that should get translated
    #[arg(short, long)]
    proj: String,

    #[arg(short, long, value_delimiter = ' ', num_args = 1..)]
    exclude: Option<Vec<String>>,

    // Personal user access token
    #[arg(long)]
    token: String,

    // Target language to translate to
    #[arg(short, long)]
    target: String

}

fn proj_files_iter(path: &str, exclude: Option<Vec<String>>) -> impl Iterator<Item = DirEntry> {
    let exclude = exclude.unwrap_or(vec![]);
    WalkDir::new(path)
        .into_iter()
        .filter_map(|f| f.ok())
        .filter(move |f| {
            let is_file = f.metadata().unwrap().is_file();

            let path = f.path().to_str().unwrap();
            let not_in_git_dir = !path.contains(".git");
            let is_strings = f.file_name().to_str().unwrap().contains(".strings");
            let not_excluded = exclude
                .iter()
                .map(|pattern| !path.to_lowercase().contains(&pattern.to_lowercase()))
                .fold(true, |acc, b| acc && b);

            is_file && not_in_git_dir && is_strings && not_excluded
        })
}

async fn search(octo: &Octocrab, text: &str, lang: &str) -> Result<Page<Code>, octocrab::Error> {
    let query = format!("\"{text}\" path:{lang}.lproj extension:strings");

    octo.search()
        .code(query.as_str())
        .page(1u32)
        .per_page(1)
        .send()
        .await
}

async fn download(octo: &Octocrab, code: &Code) -> Result<String> {
    let res = octo._get(code.url.to_string()).await?;

    let body_bytes = hyper::body::to_bytes(res.into_body()).await?;
    String::from_utf8(body_bytes.to_vec())
        .map_err(anyhow::Error::new)
}

// https://nick.groenen.me/notes/recursively-copy-files-in-rust/
fn copy_recursively<P: AsRef<Path>>(source: P, destination: P) -> Result<()> {
    fs::create_dir_all(&destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let filetype = entry.file_type()?;
        if filetype.is_dir() {
            copy_recursively(entry.path(), destination.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), destination.as_ref().join(entry.file_name()))?;
        }
    }

    Ok(())
}

#[async_std::main]
async fn main() {
    let args = Args::parse();
    let octo = OctocrabBuilder::default()
        .user_access_token(args.token)
        .add_header(http::header::ACCEPT, String::from("application/vnd.github.VERSION.raw"))
        .build()
        .unwrap();

    let res = search(&octo, "Following", &args.target).await;
    if let Err(err) = res {
        eprintln!("Failed to search for translations on GitHub: {err:?}");
        return;
    }
    let res = res.unwrap();
    for code in res.items {
        let content = download(&octo, &code).await.unwrap();
        println!("{content}");
        
        return;
    }


    let root = Path::new(&args.proj);
    let proj = source::LocalDirSource { root: &root };
    let locales: Result<Vec<_>> = proj.available_locales()
        .map(|ls| ls.collect());

    match &locales {
        Err(err) => {
            eprintln!("Failed to find localizable files in {:?}: {err}", proj.root);
            return
        }
        Ok(locales) if locales.is_empty() => {
            eprintln!("Failed to find localizable files in {:?}", proj.root);
            return
        }
        Ok(locales) => {
            let msg = locales.join(", ");
            println!("Found localizations: {msg}");
        }
    }

    let locales = locales.unwrap();
    if !locales.contains(&args.target) {
        let base_dir = proj.root.join("en.lproj");
        let target_dir = proj.root.join(args.target + ".lproj");

        if let Err(err) = copy_recursively(base_dir, target_dir) {
            eprintln!("Failed to create new directory for target language: {err}");
        }
    }

    let word = proj.translate("Activity", "de");
    println!("{:?}", word);
}

