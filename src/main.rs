use async_std;
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
    #[arg(short, long)]
    token: String

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

    let app = source::LocalDirSource { root: String::from("/Applications/Numbers.app/Contents/Resources") };
    let locales: Vec<String> = app.available_locales().unwrap().collect();
    let word = app.translate("Following", "de");
    println!("{:?}", word);
}

