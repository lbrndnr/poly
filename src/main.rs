use anyhow::Result;
use async_std;
use std::fs;
use std::path::Path;
use clap::Parser;
use http::header::HeaderName;
use octocrab::{self, Octocrab, OctocrabBuilder};
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

async fn search(octo: &Octocrab, text: &str) -> Result<(), octocrab::Error> {
    let query = format!("\"{}\" extension:strings", text);

    println!("{}", query);
    let page = octo
        .search()
        .code(query.as_str())
        .sort("indexed")
        .order("asc")
        .send()
        .await?;

    Ok(())
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

    // let files = proj_files_iter(args.proj.as_str(), args.exclude);
    // for file in files {
    //     strings::parse(file.path());
    //     println!("{:?}", file.path());
    // }

    // let localization = strings::parse("/Users/Laurin/Desktop/transmission/macosx/en.lproj/Localizable.strings").unwrap();
    // for translation in localization.translations {
        
    // }

    let octo = OctocrabBuilder::default()
        .user_access_token(args.token)
        .add_header(HeaderName::from_static("x-github-api-version"), String::from("2022-11-28"))
        .build()
        .unwrap();

    // let res = search(&octo, "Following").await;
    // println!("{:?}", res);

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

