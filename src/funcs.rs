use octocrab::models::repos::Content;
use std::process::Command;

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

        let mut output = Command::new("sh");
        output
            .arg("-c")
            .arg(format!("curl -fsSL {} | bash", &script))
            .output()
            .unwrap();
    }
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
