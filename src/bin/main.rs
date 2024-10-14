use dirs;
use ini::Ini;
use mygit::ignore::Ignore;
use mygit::object::create_tree;
use mygit::object::Object;
use mygit::object::Timestamp;
use mygit::object::User;
use std::env;
use std::fs;

fn get_user() -> Option<User> {
    let path = dirs::home_dir().unwrap().join(".gitconfig");
    let config = Ini::load_from_file(path.to_str().unwrap()).unwrap();
    let email = config.get_from(Some("user"), "email")?;
    let name = config.get_from(Some("user"), "name")?;
    Some(User::new(name, email))
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args[1] == "init" {
        fs::create_dir(".git").unwrap();
        fs::create_dir(".git/objects").unwrap();
        fs::create_dir(".git/refs").unwrap();
        fs::write(".git/HEAD", "ref: refs/heads/main\n").unwrap();
        println!("Initialized git directory")
    } else if args[1] == "cat-file" && args[2] == "-p" {
        let object = Object::from_hash(&args[3]).unwrap();
        match object {
            Object::Blob(data) => {
                println!("{}", String::from_utf8(data).unwrap());
            }
            _ => panic!("not a blob"),
        }
    } else if args[1] == "hash-object" && args[2] == "-w" {
        let filepath = env::current_dir().unwrap().join(&args[3]);
        let data = fs::read(filepath).unwrap();
        let hash = Object::Blob(data).write().unwrap();
        println!("{}", hash);
    } else if args[1] == "ls-tree" {
        let object = Object::from_hash(&args[2]).unwrap();
        match object {
            Object::Tree(entries) => {
                for entry in entries {
                    println!("{}", entry.filename());
                }
            }
            _ => panic!("not a tree"),
        }
    } else if args[1] == "write-tree" {
        let ignore = Ignore::new();
        let hash = create_tree(".", &ignore).unwrap();
        println!("{}", hash);
    } else if args[1] == "commit-tree" {
        let tree_sha = &args[2];
        let mut parents = Vec::new();
        let mut message = Option::<String>::None;
        for i in 3..args.len() {
            if args[i] == "-p" {
                parents.push(args[i + 1].to_string());
            } else if args[i] == "-m" {
                let _ = message.insert(args[i + 1].to_string());
            }
        }

        if let Some(user) = get_user() {
            let commit = Object::Commit {
                tree: tree_sha.to_string(),
                parents,
                author: user.clone(),
                author_timestamp: Timestamp::now(),
                committer: user.clone(),
                committer_timestamp: Timestamp::now(),
                message: message.unwrap(),
            };
            let hash = commit.write().unwrap();
            println!("{}", hash);
        } else {
            panic!("could not find user");
        }
    } else {
        panic!("unknown command");
    }
}
