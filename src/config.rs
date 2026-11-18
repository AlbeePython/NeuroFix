use std::net::Ipv4Addr;
use std::path::PathBuf;

use crate::dns::Resolver;

fn config_path() -> Option<PathBuf> {
    let base = std::env::var_os("APPDATA")?;
    Some(PathBuf::from(base).join("NeuroFix").join("resolvers.conf"))
}

pub fn load_custom(mut next_id: usize) -> Vec<Resolver> {
    let Some(path) = config_path() else {
        return Vec::new();
    };
    let Ok(content) = std::fs::read_to_string(path) else {
        return Vec::new();
    };

    let mut result = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = line.split(';').map(str::trim).collect();
        if parts.len() < 2 {
            continue;
        }
        let Ok(primary) = parts[1].parse::<Ipv4Addr>() else {
            continue;
        };
        let secondary = parts
            .get(2)
            .filter(|value| !value.is_empty())
            .and_then(|value| value.parse::<Ipv4Addr>().ok());
        result.push(Resolver {
            id: next_id,
            name: parts[0].to_owned(),
            primary,
            secondary,
            custom: true,
        });
        next_id += 1;
    }
    result
}

pub fn save_custom(resolvers: &[Resolver]) -> std::io::Result<()> {
    let Some(path) = config_path() else {
        return Ok(());
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut content = String::new();
    for resolver in resolvers.iter().filter(|resolver| resolver.custom) {
        let secondary = resolver
            .secondary
            .map(|ip| ip.to_string())
            .unwrap_or_default();
        content.push_str(&format!(
            "{};{};{}\n",
            resolver.name, resolver.primary, secondary
        ));
    }
    std::fs::write(path, content)
}
