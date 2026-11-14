use anyhow::{bail, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

const RUNTIME: &[(&str, &str)] = &[
    (
        "fxmanifest.lua",
        include_str!("../runtime/fiveorm/fxmanifest.lua"),
    ),
    ("init.lua", include_str!("../runtime/fiveorm/init.lua")),
    (
        "server/main.lua",
        include_str!("../runtime/fiveorm/server/main.lua"),
    ),
];

pub fn run(
    resource: Option<PathBuf>,
    resources_dir: Option<PathBuf>,
    server_cfg: Option<PathBuf>,
) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let resource_dir = resource
        .map(|path| absolute(&cwd, path))
        .or_else(|| cwd.join("fxmanifest.lua").exists().then(|| cwd.clone()));
    if let Some(ref dir) = resource_dir {
        if !dir.join("fxmanifest.lua").is_file() {
            bail!("Resource inválido: {} não possui fxmanifest.lua", dir.display());
        }
    }

    let resources = match resources_dir {
        Some(path) => absolute(&cwd, path),
        None => {
            if let Some(ref dir) = resource_dir {
                find_resources_ancestor(dir)
                    .context("Não encontrei resources/. Informe --resources-dir /caminho/resources")?
            } else {
                let candidate = cwd.join("resources");
                if !candidate.is_dir() {
                    bail!("Não encontrei resources/. Execute dentro de um resource ou use --resources-dir");
                }
                candidate
            }
        }
    };

    if !resources.is_dir() {
        bail!("Pasta de resources inexistente: {}", resources.display());
    }

    let fiveorm_dir = resources.join("fiveorm");
    install_runtime(&fiveorm_dir)?;
    println!("FiveORM runtime instalado em {}", fiveorm_dir.display());

    if let Some(ref target) = resource_dir {
        if target != &fiveorm_dir {
            let manifest = target.join("fxmanifest.lua");
            let original = fs::read_to_string(&manifest)?;
            if let Some(next) = patch_manifest(&original) {
                write_with_backup(&manifest, &next)?;
                println!("Manifest atualizado: {}", manifest.display());
            } else {
                println!("Manifest já integrado com FiveORM.");
            }
        }
    } else {
        println!("Nenhum resource de aplicação selecionado. Use --resource para integrar um fxmanifest.lua.");
    }

    let cfg = server_cfg
        .map(|path| absolute(&cwd, path))
        .or_else(|| {
            let candidate = resources.parent()?.join("server.cfg");
            candidate.is_file().then_some(candidate)
        });
    if let Some(cfg) = cfg {
        if !cfg.is_file() {
            bail!("server.cfg não encontrado: {}", cfg.display());
        }
        let original = fs::read_to_string(&cfg)?;
        match patch_server_cfg(&original) {
            Some(next) => {
                write_with_backup(&cfg, &next)?;
                println!("server.cfg atualizado: {}", cfg.display());
            }
            None => {
                if has_ensure(&original, "fiveorm") {
                    println!("server.cfg já inicia o FiveORM.");
                } else {
                    println!("ATENÇÃO: não encontrei 'ensure oxmysql' no server.cfg. Configure ensure oxmysql e ensure fiveorm nessa ordem, antes do resource consumidor.");
                }
            }
        }
    } else {
        println!("Dica: no server.cfg execute ensure oxmysql e ensure fiveorm antes do seu resource.");
    }
    Ok(())
}

fn absolute(cwd: &Path, path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        path
    } else {
        cwd.join(path)
    }
}

fn find_resources_ancestor(path: &Path) -> Option<PathBuf> {
    path.ancestors().find(|dir| dir.file_name().is_some_and(|name| name == "resources")).map(Path::to_path_buf)
}

fn install_runtime(destination: &Path) -> Result<()> {
    // Preflight all files; never overwrite an existing user-modified resource.
    for (name, contents) in RUNTIME {
        let target = destination.join(name);
        if target.exists() && fs::read_to_string(&target).as_deref() != Ok(*contents) {
            bail!("FiveORM já existe e foi modificado em {}. Faça backup e revise antes de atualizar.", target.display());
        }
    }
    for (name, contents) in RUNTIME {
        let target = destination.join(name);
        if !target.exists() {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&target, contents)
                .with_context(|| format!("Falha ao escrever {}", target.display()))?;
        }
    }
    Ok(())
}

fn patch_manifest(original: &str) -> Option<String> {
    if original.contains("@fiveorm/init.lua") {
        return None;
    }
    let marker = original
        .lines()
        .scan(0usize, |offset, line| {
            let start = *offset;
            *offset += line.len() + 1;
            Some((start, line.trim_start()))
        })
        .find(|(_, line)| line.starts_with("server_scripts")
            || line.starts_with("server_script "))
        .map(|(offset, _)| offset)
        .unwrap_or(original.len());
    let directive =
        "\n-- FiveORM: carregado antes dos scripts server do resource\ndependency 'fiveorm'\nserver_script '@fiveorm/init.lua'\n\n";
    let mut next = String::with_capacity(original.len() + directive.len());
    next.push_str(&original[..marker]);
    next.push_str(directive);
    next.push_str(&original[marker..]);
    Some(next)
}

fn has_ensure(original: &str, resource: &str) -> bool {
    original.lines().any(|line| {
        let lower = line.trim().to_ascii_lowercase();
        let mut words = lower.split_whitespace();
        matches!(words.next(), Some("ensure" | "start"))
            && words.next() == Some(resource)
    })
}

fn patch_server_cfg(original: &str) -> Option<String> {
    if has_ensure(original, "fiveorm") {
        return None;
    }
    let mut result = String::new();
    let mut inserted = false;
    for line in original.lines() {
        result.push_str(line);
        result.push('\n');
        if !inserted && {
            let lower = line.trim().to_ascii_lowercase();
            let mut words = lower.split_whitespace();
            matches!(words.next(), Some("ensure" | "start"))
                && words.next() == Some("oxmysql")
        } {
            result.push_str("ensure fiveorm\n");
            inserted = true;
        }
    }
    inserted.then_some(result)
}

fn write_with_backup(path: &Path, contents: &str) -> Result<()> {
    let file_name = path.file_name().context("Caminho inválido")?.to_string_lossy();
    let backup = path.with_file_name(format!("{file_name}.fiveorm.bak"));
    if !backup.exists() {
        fs::copy(path, &backup)
            .with_context(|| format!("Falha ao criar backup {}", backup.display()))?;
    }
    fs::write(path, contents)
        .with_context(|| format!("Falha ao atualizar {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_injects_before_server_scripts() {
        let original = "fx_version 'cerulean'\nserver_scripts { 'server.lua' }\n";
        let patched = patch_manifest(original).unwrap();
        assert!(patched.find("@fiveorm/init.lua").unwrap() < patched.find("server_scripts {").unwrap());
        assert!(patch_manifest(&patched).is_none());
    }

    #[test]
    fn config_inserts_after_oxmysql_only_once() {
        let config = "ensure oxmysql\nensure es_extended\n";
        let patched = patch_server_cfg(config).unwrap();
        assert_eq!(patched, "ensure oxmysql\nensure fiveorm\nensure es_extended\n");
        assert!(patch_server_cfg(&patched).is_none());
        assert!(patch_server_cfg("ensure es_extended\n").is_none());
    }
}
