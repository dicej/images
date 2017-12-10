extern crate failure;
extern crate regex;
extern crate rexiv2;
extern crate sha2;

// This program recursively searches the specified source directory
// for JPEG files with valid Exif.Image.DateTime tags and hard links
// them to the specified destination directory using filenames based
// on the extracted date/time.

use std::fs::{hard_link, read_dir, File};
use std::path::Path;
use std::io::Read;
use rexiv2::Metadata;
use failure::Error;
use regex::Regex;
use sha2::{Digest, Sha256};

fn hash<P: AsRef<Path>>(path: P) -> Result<Vec<u8>, Error> {
    let mut hasher = Sha256::default();
    let mut buffer = Vec::new();
    File::open(path)?.read_to_end(&mut buffer)?;
    hasher.input(&buffer);
    Ok(hasher.result().into_iter().collect())
}

fn run<P: AsRef<Path>>(src: P, dst: &str, pattern: &Regex) -> Result<(), Error> {
    for entry in read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            if let Some(name) = entry.file_name().to_str() {
                let lowercase = name.to_lowercase();
                if lowercase.ends_with(".jpg") || lowercase.ends_with(".jpeg") {
                    if let Some(name) = Metadata::new_from_path(&path)
                        .and_then(|m| m.get_tag_string("Exif.Image.DateTime"))
                        .ok()
                        .and_then(|s| {
                            pattern.captures(&s).map(|c| {
                                format!(
                                    "{}-{}-{}_{}_{}_{}",
                                    &c[1],
                                    &c[2],
                                    &c[3],
                                    &c[4],
                                    &c[5],
                                    &c[6]
                                )
                            })
                        }) {
                        let mut buf = Path::new(dst).to_path_buf();
                        buf.push(&format!("{}.jpeg", name));

                        if buf.is_file() {
                            println!("{} already exists", buf.to_str().unwrap_or("<unprintable>"));
                            // this is a bit silly -- if we're going
                            // to read the entire files in anyway, we
                            // might as well just compare them
                            // byte-by-byte instead of calculating
                            // hashes and comparing those.
                            if hash(&buf)? == hash(&path)? {
                                println!("hashes map; skipping");
                                continue;
                            }

                            let mut i = 1;
                            while buf.is_file() {
                                buf = Path::new(dst).to_path_buf();
                                buf.push(&format!("{}-{}.jpeg", name, i));
                                i += 1;
                            }

                            println!("using {} instead", buf.to_str().unwrap_or("<unprintable>"));
                        }

                        println!(
                            "hard link {} to {}",
                            path.to_str().unwrap_or("<unprintable>"),
                            buf.to_str().unwrap_or("<unprintable>"),
                        );

                        hard_link(path, buf)?;
                    }
                }
            }
        } else if path.is_dir() {
            run(&path, dst, pattern)?;
        }
    }

    Ok(())
}

fn main() {
    let mut args = std::env::args();

    let usage = format!(
        "usage: {} <src directory> <dst directory>",
        args.next().expect("program has no name?")
    );

    let src = args.next().expect(&usage);
    let dst = args.next().expect(&usage);

    let pattern = Regex::new(r"(\d{4}):(\d{2}):(\d{2}) (\d{2}):(\d{2}):(\d{2})").unwrap();

    if let Err(e) = run(&src, &dst, &pattern) {
        eprintln!("exit on error: {:?}", e)
    }
}
