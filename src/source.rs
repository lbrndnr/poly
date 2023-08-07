use std::fs;
use std::path::Path;
use anyhow::Result;
use crate::strings::*;

pub struct LocalDirSource {

    pub root: String

}

impl LocalDirSource {

    pub fn available_locales(&self) -> Result<impl Iterator<Item = String>> {
        let res = fs::read_dir(&self.root)
            .unwrap()
            .filter_map(|p| {
                let file_name = p.unwrap().file_name();
                let string = file_name.into_string().unwrap();
                let res = string.strip_suffix(".lproj");
                
                res.map(|s| s.to_owned())
            });

        Ok(res)
    }

    pub fn translate(&self, word: &str, target_locale: &str) -> Result<Option<String>> {
        let path = Path::new(&self.root).join("en.lproj");
        let id = self.translate_in_dir(word, path, true)?;

        match id {
            Some(id) => {
                let locale_dir = format!("{}.lproj", target_locale);
                let path = Path::new(&self.root).join(locale_dir);
                self.translate_in_dir(&id, path, false)
            }
            None => Ok(None)
        }
    }

    fn translate_in_dir<P: AsRef<Path>>(&self, word: &str, dir: P, inversed: bool) -> Result<Option<String>> {
        let path = fs::read_dir(dir)
            .unwrap()
            .map(|p| p.unwrap().path())
            .find(|p| {
                let is_strings = p.extension().map_or(false, |e| e == "strings");
                if is_strings { 
                    let loc = parse(p, inversed);
                    loc.map_or(false, |l| l.translations.contains_key(word)) 
                }
                else { false }
            });

        match path {
            Some(path) => Ok(parse(path, inversed)?.translations.get(word).map(|t| t.target.clone())),
            None => Ok(None)
        }
    }

}