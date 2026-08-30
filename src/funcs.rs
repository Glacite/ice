use octocrab::models::repos::Content;
use reqwest;
//use serde::Deserialize;
use std::{fs, net::TcpStream, path::PathBuf, process::Command};
use strace_parse::raw::{Call, parse};

/*
#[derive(Deserialize)]
struct Meta {
    dependencies: Option<Dependencies>,
}

#[derive(Deserialize)]
struct Dependencies {
    setup: Vec<String>,
    runtime: Vec<String>,
}
*/

pub const PKGSPATH: &str = "/usr/lib/ice/packages/";

pub async fn contents() -> Vec<Content> {
    octocrab::instance()
        .repos("Glacite", "packages")
        .get_content()
        .r#ref("main")
        .send()
        .await
        .unwrap()
        .take_items()
}

pub async fn list() -> Vec<String> {
    let mut vec: Vec<String> = Vec::new();

    for c in contents().await {
        if c.r#type == "dir" {
            vec.push(c.name)
        }
    }

    vec
}

pub async fn search(pkg: impl Into<String>) -> bool {
    list().await.contains(&pkg.into())
}

pub async fn install(pkg: impl Into<String>) {
    let pkg = pkg.into();

    if search(&pkg).await {
        let content = octocrab::instance()
            .repos("Glacite", "packages")
            .get_content()
            .r#ref("main")
            .path(&pkg)
            .send()
            .await
            .unwrap()
            .take_items();

        let mut output = Command::new("strace");
        let strace = output
            .arg("-f")
            .arg("-qq")
            .arg("-e")
            .arg("trace=mkdir")
            .arg("-e")
            .arg("signal=!SIGCHLD")
            .arg("sh")
            .arg("-c")
            .arg(format!("{{\n{}\n}} >/dev/null 2>&1", script(content).await))
            .output()
            .unwrap()
            .stderr;

        let mut remove = String::from("#!/bin/bash\n");

        for syscall in parse(strace.as_slice()) {
            let (call, args) = match syscall {
                Ok(c) => match c.call {
                    Call::Generic(gc) => (
                        gc.call.split_whitespace().last().unwrap().to_string(),
                        gc.args,
                    ),
                    _ => continue,
                },
                Err(_) => continue,
            };

            match call.as_str() {
                "mkdir" => remove = format!("{remove}rm -r {}\n", args[0]),
                _ => panic!("Unknown call"),
            }
        }

        let pkgpath = format!("{PKGSPATH}{}/", &pkg);

        fs::create_dir_all(&pkgpath).unwrap();
        fs::write(format!("{}remove.sh", &pkgpath), &remove).unwrap();
    }
}

pub fn remove(pkg: impl Into<String>) {
    let pkg = pkg.into();
    let pkgpath = format!("{PKGSPATH}{}/", &pkg);
    let script = format!("{pkgpath}remove.sh");

    let mut command = Command::new("sh");
    command.arg(&script).output().unwrap();

    fs::remove_dir_all(pkgpath).unwrap();
}

pub fn is_installed(pkg: impl Into<String>) -> bool {
    let pkg = pkg.into();
    match fs::read_dir(PKGSPATH) {
        Ok(mut d) => match d.find(|f| f.as_ref().unwrap().file_name().to_str().unwrap() == &pkg) {
            Some(_) => true,
            None => false,
        },
        Err(_) => false,
    }
}

pub fn github_ping() -> bool {
    TcpStream::connect("github.com:443").is_ok()
}

async fn script(contents: Vec<Content>) -> String {
    let url = contents
        .iter()
        .find(|c| c.r#name == "package.sh")
        .unwrap()
        .download_url
        .clone()
        .unwrap();

    reqwest::get(url).await.unwrap().text().await.unwrap()
}

pub async fn fetch(url: impl Into<String>, output: Option<PathBuf>) -> Result<(), String> {
    let url = url.into();
    let file = match reqwest::get(&url).await {
        Ok(r) => r.bytes().await.unwrap(),
        Err(e) => return Err(e.to_string()),
    };

    let name = name_from_url(&url);

    let mut dir: PathBuf;
    match output {
        None => {
            dir = std::env::current_dir().unwrap();
            dir.push(name);
        }
        Some(p) => {
            dir = p;
            match &dir.is_dir() {
                true => {
                    fs::create_dir_all(&dir).unwrap();
                    dir.push(name);
                }
                false => {
                    fs::create_dir_all(&dir.parent().unwrap()).unwrap();
                }
            }
        }
    }
    fs::write(&dir, &file).unwrap();

    Ok(())
}

fn name_from_url(url: impl Into<String>) -> String {
    url.into().split("/").last().unwrap().into()
}
