// use ron::de::SpannedError;
// use serde::{Deserialize, Serialize};
// #[cfg(not(target_family = "wasm"))]
// use crate::dir;
// use serde::{Deserialize, Serialize};
#[cfg(not(target_family = "wasm"))]
use std::path::PathBuf;

// use crate::map::Map;

// const

#[cfg(not(target_family = "wasm"))]
use directories::ProjectDirs;

#[cfg(not(target_family = "wasm"))]
/// returns the ProjectDirs struct from the directories crate with the proper identifier for the
/// game
pub fn project_dirs() -> ProjectDirs {
    #[cfg(test)]
    let dirs = ProjectDirs::from("com", "TilesRUs", "gnome-io-test").unwrap();
    #[cfg(not(test))]
    let dirs = ProjectDirs::from("com", "TilesRUs", "gnome-io").unwrap();
    dirs
}

#[derive(Debug)]
pub enum SaveError {
    #[allow(unused)]
    ReadFile(std::io::Error),

    #[cfg(target_family = "wasm")]
    MissingLocalStorage,
    // #[allow(unused)]
    // Deserialize(SpannedError),
    // #[allow(unused)]
    // Serialize(ron::Error),
}

impl From<std::io::Error> for SaveError {
    fn from(err: std::io::Error) -> Self {
        SaveError::ReadFile(err)
    }
}

// impl From<SpannedError> for SaveError {
//     fn from(err: SpannedError) -> Self {
//         SaveError::Deserialize(err)
//     }
// }

// impl From<ron::Error> for SaveError {
//     fn from(err: ron::Error) -> Self {
//         SaveError::Serialize(err)
//     }
// }

pub type LoadResult<T> = Result<T, SaveError>;
pub type SaveResult = Result<(), SaveError>;

pub struct Storage {
    key: &'static str,
    ext: &'static str,
}

impl Storage {
    pub const fn new(key: &'static str, ext: &'static str) -> Self {
        Self { key, ext }
    }

    #[cfg(not(target_family = "wasm"))]
    pub fn save_exists(&self) -> bool {
        std::fs::exists(self.determine_save_path()).unwrap()
    }

    #[cfg(target_family = "wasm")]
    pub fn save_exists(&self) -> bool {
        quad_storage::STORAGE.lock().unwrap().len() > 0
    }

    // pub fn load_ron<'a, T>(&self) -> LoadResult<T>
    // where
    //     T: Deserialize<'a> + Clone,
    // {
    //     // let str = ;

    //     let str = self.load()?;
    //     let res = Ok(ron::from_str::<T>(str.as_str())?.clone());
    //     drop(str);
    //     res
    // }

    #[cfg(not(target_family = "wasm"))]
    pub fn load(&self) -> Result<String, SaveError> {
        let save_path = self.determine_save_path();

        log::info!("load {:?}", save_path.as_os_str());

        Ok(std::fs::read_to_string(save_path)?)
        // Ok(ron::from_str(toml_str.as_str())?)
    }

    #[cfg(not(target_family = "wasm"))]
    fn determine_save_path(&self) -> PathBuf {
        let project_dirs = project_dirs();
        let save_dir = project_dirs.data_local_dir();
        std::fs::create_dir_all(save_dir).unwrap();
        let mut save_path = PathBuf::from(save_dir);
        save_path.push(format!("{}{}", self.key, self.ext));
        // save_path.push(self.ext);
        save_path
    }

    #[cfg(target_family = "wasm")]
    pub fn load(&self) -> Result<String, SaveError> {
        let storage = &mut quad_storage::STORAGE.lock().unwrap();
        storage.get(self.key).ok_or(SaveError::MissingLocalStorage)
        // Ok(ron::from_str(wasm_save.as_str()).unwrap())
    }

    /// writes the save to local storage
    #[cfg(target_family = "wasm")]
    pub fn save(&self, str: &str) -> SaveResult {
        // TODO: Solve unwrap
        let storage = &mut quad_storage::STORAGE.lock().unwrap();
        storage.set(self.key, str);
        Ok(())
    }

    #[cfg(not(target_family = "wasm"))]
    /// writes the save to disk
    pub fn save(&self, str: &str) -> SaveResult {
        Ok(std::fs::write(self.determine_save_path(), str)?)
    }

    // /// returns the save data in RON format as a pretty string
    // pub fn save_as_ron<T>(&self, value: &T) -> SaveResult
    // where
    //     T: ?Sized + Serialize,
    // {
    //     self.save(ron::ser::to_string_pretty(value, ron::ser::PrettyConfig::default())?.as_str())
    // }
}

#[cfg(test)]
mod storage_tests {
    use super::*;

    #[test]
    fn test_storage() {
        let storage = Storage::new("test", ".ron");

        let _ = std::fs::remove_file(storage.determine_save_path());

        assert!(!storage.save_exists());
        assert!(storage.load().is_err());

        let my_data = "1234";

        assert!(storage.save(my_data).is_ok());
        assert!(storage.save_exists());

        assert_eq!(storage.load().unwrap().as_str(), my_data);
    }
}
