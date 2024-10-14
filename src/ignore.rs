use std::{fs, path};

pub struct Ignore {
    entries: Vec<String>,
}

impl Ignore {
    pub fn new() -> Ignore {
        let mut entries = Vec::new();
        if let Ok(content) = fs::read(".gitignore") {
            entries = content
                .split(|&b| b == b'\n')
                .filter(|line| !line.starts_with(b"#") && line.len() > 0)
                .map(|line| {
                    path::absolute(String::from_utf8(line.to_vec()).unwrap())
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .trim_end_matches('/')
                        .to_string()
                })
                .collect();
        }

        entries.push(
            path::absolute(".git")
                .unwrap()
                .to_str()
                .unwrap()
                .to_string(),
        );

        Ignore { entries }
    }

    pub fn contains(&self, path: &str) -> bool {
        let abspath = path::absolute(path);
        match abspath {
            Err(_) => return false,
            Ok(abspath) => {
                if let Some(abspath) = abspath.to_str() {
                    return self.entries.contains(&abspath.to_string());
                } else {
                    return false;
                }
            }
        }
    }
}
