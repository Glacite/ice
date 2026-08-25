use octocrab::models::repos::Content;
use std::{fs, process::Command};
use strace_parse::raw::{Call, parse};

pub static PKGSPATH: &str = "/usr/lib/ice/packages/";

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

        let script = script(content).await;

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
            .arg(format!("curl -fsSL {} | bash &> /dev/null", &script))
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
    let script = format!("{pkgpath}{}remove.sh", &pkg);

    let mut command = Command::new("sh");
    command.arg(&script).output().unwrap();

    fs::remove_dir_all(pkgpath).unwrap();
}

async fn script(contents: Vec<Content>) -> String {
    contents
        .iter()
        .find(|c| c.r#name == "package.sh")
        .unwrap()
        .download_url
        .clone()
        .unwrap()
}
