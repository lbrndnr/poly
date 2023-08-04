use std::fs;
use anyhow::{Result};

pub trait Source {

    fn available_locales(&self) -> Result<Vec<&str>>;
    fn translate(&self, word: &str, target_locale: &str) -> Result<&str>;

}

pub struct LocalDirSource {

    pub root: String

}

impl Source for LocalDirSource {

    fn available_locales(&self) -> Result<Vec<&str>> {
        fs::read_dir(&self.root)
            .unwrap()
            .filter(|p| p.unwrap().file_name().to_str().unwrap().contains(".lproj"));

        Ok(vec!["en"])
    }

    fn translate(&self, word: &str, target_locale: &str) -> Result<&str> {
        Ok("haha")
    }

}