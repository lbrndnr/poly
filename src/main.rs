use anyhow::{Error, Result};
use async_std;
use strings::Localization;
use std::fs;
use std::path::Path;
use hyper;
use clap::Parser;
use octocrab::{OctocrabBuilder, Octocrab, Page};
use octocrab::models::*;
use walkdir::{WalkDir, DirEntry};

mod strings;
mod proj;

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
        .send()
        .await
}

async fn download(octo: &Octocrab, code: &Code) -> Result<Localization> {
    let locale = strings::resolve_path_locale(code.url.as_ref()).ok_or(Error::msg("Could not resolve locale using path"))?;
    let res = octo._get(code.url.to_string()).await?;
    let body_bytes = hyper::body::to_bytes(res.into_body()).await?;

    String::from_utf8(body_bytes.to_vec())
        .map_err(Error::new)
        .and_then(|c| Localization::from_params(locale, c.as_str(), false))
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

    let root = Path::new(&args.proj);
    if !root.is_dir() {
        eprintln!("The project root is not a directory.");
        return;
    }

    let proj = proj::Project { root: &root };
    let locales: Vec<_> = proj.available_locales().collect();

    if locales.len() > 0 {
        let msg = locales.join(", ");
        println!("Found localizations: {msg}");
    }
    else {
        eprintln!("Failed to find localizable files in {:?}", proj.root);
        return;
    }

    if !locales.contains(&args.target) {
        let base_dir = proj.root.join("en.lproj");
        let target_dir = proj.root.join(args.target.clone() + ".lproj");

        if let Err(err) = copy_recursively(base_dir, target_dir) {
            eprintln!("Failed to create new directory for target language: {err}");
        }
    }

    let base = proj.localizations_for_locale("en").next().unwrap();
    let localizations = proj.localizations_for_locale(&args.target);
    for loc in localizations {
        for (id, value) in loc.translations {
            let word = &base.translations.get(&id).unwrap().target;
            let res = search(&octo, &word, &args.target).await;
            if let Err(err) = res {
                eprintln!("Failed to search for translations on GitHub: {err:?}");
                return;
            }

            let res = res.unwrap();
            if let Some(code) = res.items.first() {
                let remote_loc = download(&octo, &code).await.unwrap();
                println!("{}", word);
                println!("{:?}", remote_loc.translations.keys());
                let translation = remote_loc.translations.get(word.as_str());
                if let Some(translation) = translation {
                    println!("{word} -> {}", translation.target);
                    continue;
                }
            }

            println!("Did not find any results for {word} on github.")
        }
    }
}

