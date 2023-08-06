use anyhow::Error;
use std::ffi::OsString;
use std::fs::File;
use std::io::Read;
use std::os::unix::prelude::OsStringExt;
use std::path::Path;
use std::collections::HashMap;

pub struct Localization {
    pub locale: String,
    pub translations: HashMap<String, Translation>
}

pub struct Translation {
    pub comment: String,
    pub source: String,
    pub target: String
}

#[derive(PartialEq)]
enum ParsingCursor {
    Whitespace,
    Comment,
    Source,
    Target
}

fn target_locale_of_file(path: &str) -> Result<&str, Error> {
    let comps: Vec<&str> = Path::new(path)
    .components()
    .map(|c| c.as_os_str().to_str().unwrap())
    .filter(|c| c.ends_with(".lproj"))
    .collect();

    Ok(comps[0].strip_suffix(".lproj").unwrap())
}

pub fn parse<P: AsRef<Path>>(path: P, inversed: bool) -> Result<Localization, Error> {
    let file = File::open(&path)?;
    let mut reader = encoding_rs_io::DecodeReaderBytesBuilder::new()
        .encoding(Some(encoding_rs::UTF_16LE))
        .build(file);

    // let mut content = Vec::new();
    // reader.read_to_end(&mut content);

    // let content = OsString::from_vec(content);
    let mut content = String::new();
    reader.read_to_string(&mut content)?;

    let mut translations = HashMap::new();
    let mut comment = String::new();
    // let mut source = String::new();
    // let mut target = String::new();
    let mut cursor = ParsingCursor::Whitespace;

    for line in content.lines() {
        if line.len() == 0 { continue }

        if line.starts_with("//") || line.starts_with("/*") || cursor == ParsingCursor::Comment {
            comment.push_str(line);
            cursor = if line.ends_with("*/") { ParsingCursor::Whitespace } else { ParsingCursor::Comment };
            continue;
        }

        let text: Vec<&str> = line
        .split("\"")
        .filter(|s| ! (s.is_empty() || *s == ";" || (*s).contains("=")))
        .collect();
    
        if text.len() == 2 {
            let source = String::from(text[0]);
            let target = String::from(text[1]);

            println!("{} - {}", source, target);

            if inversed {
                translations.insert(target.clone(), Translation { comment: comment.clone(), target, source });
            }
            else {
                translations.insert(source.clone(), Translation { comment: comment.clone(), source, target });
            }
        }
    }

    let locale = target_locale_of_file(path.as_ref().to_str().unwrap())?;
    Ok(Localization { locale: locale.to_owned(), translations })
}