use anyhow::Error;
use std::ffi::OsStr;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::collections::HashMap;

#[derive(Clone)]
pub struct Localization {
    pub locale: String,
    pub translations: HashMap<String, Translation>
}

#[derive(Clone)]
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

impl Localization {

    pub fn from_params(locale: &str, content: &str, inversed: bool) -> Result<Self, Error> {
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
                let mut source = String::from(text[0]);
                let mut target = String::from(text[1]);
                if inversed {
                    (source, target) = (target, source)
                }
    
                translations.insert(source.to_lowercase(), Translation { comment: comment.clone(), source, target });
            }
        }
    
        Ok(Localization { locale: locale.to_owned(), translations })
    }

    pub fn from_file(path: impl AsRef<Path>, inversed: bool) -> Result<Self, Error> {
        let mut file = File::open(&path)?;
        let mut content = String::new();
        file.read_to_string(&mut content)?;

        let locale = resolve_path_locale(path.as_ref()).ok_or(Error::msg("Could not resolve locale using path"))?;
        Localization::from_params(locale, &content, inversed)
    }

    pub fn write_to_file(path: impl AsRef<Path>) -> Result<(), Error> {
        Ok(())
    }

}

pub fn resolve_path_locale<S: AsRef<OsStr> + ?Sized>(path: &S) -> Option<&str> {
    Path::new(path)
        .components()
        .rev()
        .find_map(|c| {
            c.as_os_str().to_str().and_then(|n| n.strip_suffix(".lproj"))
        })
}