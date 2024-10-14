use crypto::digest::Digest;
use crypto::sha1::Sha1;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use std::fmt;
use std::fmt::Display;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use crate::ignore::Ignore;

#[derive(Debug, Clone)]
pub struct InvalidObjectFormat;

impl fmt::Display for InvalidObjectFormat {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "invalid object format")
    }
}

impl std::error::Error for InvalidObjectFormat {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

#[derive(Clone)]
pub struct User {
    name: String,
    email: String,
}

impl Display for User {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} <{}>", self.name, self.email)
    }
}

impl User {
    pub fn new(name: &str, email: &str) -> User {
        User {
            name: name.to_string(),
            email: email.to_string(),
        }
    }
}

pub struct Timestamp {
    seconds: i64,
    offset: i32,
}

impl Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sign = if self.offset < 0 { '-' } else { '+' };
        let offset = self.offset.abs();
        let hours = offset / 3600;
        let minutes = offset % 3600;
        write!(f, "{} {sign}{hours:02}{minutes:02}", self.seconds)
    }
}

impl Timestamp {
    pub fn now() -> Timestamp {
        let now = chrono::Local::now();
        let offset = now.offset().local_minus_utc();
        Timestamp {
            seconds: now.timestamp(),
            offset,
        }
    }
}

pub struct Entry {
    mode: String,
    filename: String,
    hash: String,
}

impl Entry {
    pub fn mode(&self) -> &str {
        &self.mode
    }

    pub fn filename(&self) -> &str {
        &self.filename
    }

    pub fn hash(&self) -> &str {
        &self.hash
    }
}

pub enum Object {
    Blob(Vec<u8>),
    Tree(Vec<Entry>),
    Commit {
        tree: String,
        parents: Vec<String>,
        author: User,
        author_timestamp: Timestamp,
        committer: User,
        committer_timestamp: Timestamp,
        message: String,
    },
}

impl Object {
    pub fn from_hash(hash: &str) -> Result<Object, Box<dyn std::error::Error + 'static>> {
        let path = format!(".git/objects/{}/{}", &hash[..2], &hash[2..]);
        let mut reader = BufReader::new(ZlibDecoder::new(fs::File::open(path)?));

        let mut buf = Vec::new();
        reader.read_until(b' ', &mut buf)?;
        buf.pop();

        let object_type = String::from_utf8(buf)?;
        match object_type.as_str() {
            "blob" => {
                let mut data = Vec::new();
                reader.read_to_end(&mut data)?;
                Ok(Object::Blob(data))
            }
            "tree" => {
                let mut entries = Vec::new();
                while !reader.fill_buf()?.is_empty() {
                    let mut mode = Vec::new();
                    reader.read_until(b' ', &mut mode)?;
                    mode.pop();

                    let mut filename = Vec::new();
                    reader.read_until(b'\0', &mut filename)?;
                    filename.pop();

                    let mut hash = [0; 20];
                    reader.read_exact(&mut hash)?;

                    entries.push(Entry {
                        mode: String::from_utf8(mode)?,
                        filename: String::from_utf8(filename)?,
                        hash: hash.iter().map(|b| format!("{:02x}", b)).collect(),
                    });
                }
                Ok(Object::Tree(entries))
            }
            _ => Err(Box::new(InvalidObjectFormat)),
        }
    }
    pub fn write(&self) -> Result<String, Box<dyn std::error::Error + 'static>> {
        let content = match self {
            Object::Blob(data) => [format!("blob {}\0", data.len()).as_bytes(), &data].concat(),
            Object::Tree(entries) => {
                let mut tree_content = Vec::new();
                for entry in entries {
                    tree_content.extend_from_slice(entry.mode.as_bytes());
                    tree_content.push(b' ');
                    tree_content.extend_from_slice(entry.filename.as_bytes());
                    tree_content.push(b'\0');
                    let hex_bytes = (0..entry.hash.len())
                        .step_by(2)
                        .map(|i| u8::from_str_radix(&entry.hash[i..i + 2], 16))
                        .collect::<Result<Vec<_>, _>>()?;
                    tree_content.extend_from_slice(&hex_bytes);
                }
                [
                    format!("tree {}\0", tree_content.len()).as_bytes(),
                    &tree_content,
                ]
                .concat()
            }
            Object::Commit {
                tree,
                parents,
                author,
                author_timestamp,
                committer,
                committer_timestamp,
                message,
            } => {
                let commit_content = format!(
                    "tree {}\n\
                  {}\
                  author {} {}\n\
                  committer {} {}\n\n\
                  {}\n",
                    tree,
                    if parents.len() > 0 {
                        parents
                            .iter()
                            .map(|p| format!("parent {}", p))
                            .collect::<Vec<String>>()
                            .join("\n")
                            + "\n"
                    } else {
                        "".to_string()
                    },
                    author,
                    author_timestamp,
                    committer,
                    committer_timestamp,
                    message
                );

                [
                    format!("commit {}\0", commit_content.bytes().len()).as_bytes(),
                    commit_content.as_bytes(),
                ]
                .concat()
            }
        };
        let hash = {
            let mut hasher = Sha1::new();
            hasher.input(&content);
            hasher.result_str()
        };
        let dir = format!(".git/objects/{}", &hash[..2]);
        let filepath = Path::new(&dir).join(&hash[2..]);
        if filepath.exists() {
            return Ok(hash);
        }

        fs::create_dir_all(&dir)?;
        let mut encoder = ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(&content)?;
        let compressed = encoder.finish()?;
        fs::write(Path::new(&dir).join(&hash[2..]), &compressed)?;
        Ok(hash)
    }
}

pub fn create_tree(
    path: &str,
    ignore: &Ignore,
) -> Result<String, Box<dyn std::error::Error + 'static>> {
    let fs_entries = fs::read_dir(path)?;
    let mut entries = Vec::new();

    for fs_entry in fs_entries {
        let fs_entry = fs_entry?;
        let path = fs_entry.path();
        let filepath = path.to_str().ok_or(InvalidObjectFormat)?;
        let filename = fs_entry
            .file_name()
            .to_str()
            .ok_or(InvalidObjectFormat)?
            .to_string();

        if ignore.contains(filepath) {
            continue;
        }

        if fs_entry.file_type()?.is_dir() {
            entries.push(Entry {
                mode: "040000".to_string(),
                filename,
                hash: create_tree(filepath, ignore)?,
            });
            continue;
        }

        let metadata = fs_entry.metadata()?;
        if metadata.is_symlink() {
            continue;
        }

        let is_executable = metadata.permissions().mode() & 0o111 != 0;
        entries.push(Entry {
            mode: if is_executable { "100755" } else { "100644" }.to_string(),
            filename,
            hash: Object::Blob(fs::read(path).or(Err(InvalidObjectFormat))?).write()?,
        });
    }

    entries.sort_by(|a, b| a.filename.cmp(&b.filename));

    Object::Tree(entries).write()
}
