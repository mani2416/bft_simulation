extern crate glob;
extern crate log;

use self::log::{debug, error};
use glob::glob;
use std::fs;
use std::io;
use std::io::{BufRead, Read, Write};
use std::path;
use std::time::SystemTime;

///
pub struct FileHelper {}

impl FileHelper {
    /// Deletes all files matching pattern
    pub fn delete_matching_files(pattern: &str) -> Result<(), io::Error> {
        debug!("Deleting files matching {}", pattern);
        for entry in glob(&pattern).expect("Failed to read glob pattern") {
            match entry {
                Ok(path) => {
                    debug!("{:?}", path.display());
                    if path.is_dir() {
                        fs::remove_dir_all(path)?;
                    } else if path.is_file() {
                        fs::remove_file(path)?;
                    }
                }
                Err(e) => error!("{:?}", e),
            }
        }
        Ok(())
    }

    /// Deletes a directory
    pub fn delete_dir(dir: &str) -> Result<(), io::Error> {
        debug!("Deleting directory: {}", dir);
        fs::remove_dir_all(dir)?;
        Ok(())
    }

    /// Writes the content to a file (Creates it, if it doesn't exist and overwrites, if it does)
    pub fn write_to_file(file_path: &str, content: &str) -> Result<(), io::Error> {
        debug!("Creating new file: {}", file_path);
        let mut only_path = String::new();
        let length = file_path.split('/').count();
        for (i, v) in file_path.split('/').enumerate() {
            if v.is_empty() || i == length - 1 {
                continue;
            }
            only_path = only_path + v + "/";
        }

        // debug!("Creating path '{}'", only_path);
        fs::create_dir_all(only_path)?;
        let mut file = fs::File::create(file_path)?;
        file.write_all(content.as_bytes())?;
        Ok(())
    }

    /// Removes a file
    pub fn remove_file(file_path: &str) -> Result<(), io::Error> {
        debug!("Deleting file: {}", file_path);
        fs::remove_file(file_path)?;
        Ok(())
    }

    /// Appends the content to a file (Creates it, if it doesn't exist)
    pub fn append_to_file(file_path: &str, content: &str) -> Result<(), io::Error> {
        debug!("Writing to file: {}", file_path);
        let mut file = fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(file_path)?;
        file.write_all(content.as_bytes())?;
        Ok(())
    }

    /// Appends the content to a file and adds a newline afterwards (Creates the file, if it doesn't exist)
    pub fn append_to_file_ln(file_path: &str, content: &str) -> Result<(), io::Error> {
        debug!("Writing to file: {}", file_path);
        let mut file = fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(file_path)?;
        file.write_all(content.as_bytes())?;
        file.write_all("\n".as_bytes())?;
        Ok(())
    }

    /// Returns all files in a directory
    pub fn get_all_files_in(path: &str) -> Result<Vec<fs::DirEntry>, io::Error> {
        fs::read_dir(&path)?.collect()
    }

    /// Creates an empty file
    pub fn create_file(file_path: &str) -> Result<(), io::Error> {
        let _ = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(file_path)?;
        Ok(())
    }

    /// Returns the 'SystemTime' of the 'last modified' field of the file
    pub fn get_last_modified(file_path: &str) -> Result<SystemTime, io::Error> {
        let metadata = fs::metadata(file_path)?;
        metadata.modified()
    }

    /// Copies a file
    pub fn copy_file(file_from: &str, file_to: &str) -> io::Result<()> {
        debug!("Copying {} to {}", file_from, file_to);
        fs::copy(file_from, file_to)?;
        Ok(())
    }

    /// Renames a file
    pub fn rename_file(file_from: &str, file_to: &str) -> Result<(), io::Error> {
        debug!("Renaming {} to {}", file_from, file_to);
        fs::rename(file_from, file_to)
    }

    /// Returns true if the file exists
    pub fn file_exists(file_path: &str) -> bool {
        path::Path::new(file_path).exists()
    }

    /// Reads the content of a file and returns a String (using String::from_utf8())
    pub fn read_file_to_string(file_path: &str) -> Result<String, io::Error> {
        debug!("Reading content of {}", file_path);
        let mut content: Vec<u8> = Vec::new();
        let mut file = fs::OpenOptions::new().read(true).open(file_path)?;
        let _ = file.read_to_end(&mut content)?;
        match String::from_utf8(content) {
            Ok(s) => Ok(s),
            Err(_) => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "UTF 8 conversion failed",
            )),
        }
    }

    /// Reads the content of a file from a given position and return a String (using String::from_utf8())
    pub fn read_file_from_position_to_string(
        file_path: &str,
        idx: usize,
        content: &mut String,
    ) -> Result<usize, io::Error> {
        let mut all: Vec<u8> = Vec::new();
        let mut file = fs::OpenOptions::new().read(true).open(file_path)?;
        let _ = file.read_to_end(&mut all)?;

        if let Some(part) = all.get(idx..) {
            match String::from_utf8(part.to_vec()) {
                Ok(s) => *content = s,
                Err(_) => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "UTF 8 conversion failed",
                    ));
                }
            }
        } else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "File could not be read from the given position",
            ));
        }
        Ok(all.len())
    }

    /// Returns the amount of lines of a file
    pub fn amount_of_lines_in_file(file_path: &str) -> Result<usize, io::Error> {
        let file = fs::File::open(file_path)?;
        Ok(io::BufReader::new(file).lines().count())
    }
}
