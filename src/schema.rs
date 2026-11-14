use anyhow::{bail, Context, Result};
use clap::ValueEnum;
use dialoguer::Select;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;
use std::io::IsTerminal;
use std::path::Path;

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum Framework {
    Vrpex,
    Vrp,
    Esx,
    Creative,
    #[value(alias = "qbone")]
    Qbcore,
    Qbox,
    Standalone,
}

impl Framework {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Vrpex => "vrpex",
            Self::Vrp => "vrp",
            Self::Esx => "esx",
            Self::Creative => "creative",
            Self::Qbcore => "qbcore",
            Self::Qbox => "qbox",
            Self::Standalone => "standalone",
        }
    }
}

const FRAMEWORKS: &[Framework] = &[
    Framework::Vrpex,
    Framework::Vrp,
    Framework::Esx,
    Framework::Creative,
    Framework::Qbcore,
    Framework::Qbox,
    Framework::Standalone,
];

pub fn init(path: &Path, framework: Option<Framework>) -> Result<()> {
    if path.exists() {
        println!("Schema já existe em {}. Nenhum arquivo foi sobrescrito.", path.display());
        return validate(path);
    }

    let framework = match framework {
        Some(value) => value,
        None => {
            if !std::io::stdin().is_terminal() {
                bail!("Terminal interativo indisponível. Execute com --framework vrpex|vrp|esx|creative|qbcore|qbox|standalone");
            }
            let labels = [
                "vRPex",
                "vRP",
                "ESX",
                "Creative",
                "QBCore",
                "Qbox",
                "Standalone / custom",
            ];
            let selected = Select::new()
                .with_prompt("Qual framework FiveM você utiliza?")
                .items(&labels)
                .default(0)
                .interact()?;
            FRAMEWORKS[selected]
        }
    };

    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Não foi possível criar {}", parent.display()))?;
        }
    }

    // Framework selection is metadata, not an assumption about a server's actual tables.
    let body = format!(
        "# FiveORM v0.1 — schema inicial (nenhuma tabela é criada automaticamente)\n\
         version: 1\n\
         project:\n\
           framework: {}\n\
         database:\n\
           provider: mysql\n\
           url_env: DATABASE_URL\n\
         models: {{}}\n",
        framework.as_str()
    );
    fs::write(path, body).with_context(|| format!("Falha ao gravar {}", path.display()))?;
    println!("Criado {} (framework: {}).", path.display(), framework.as_str());
    println!("Próximo passo: fiveorm validate && fiveorm setup");
    Ok(())
}

#[derive(Debug, Deserialize)]
struct Schema {
    version: u32,
    project: Project,
    database: Database,
    #[serde(default)]
    models: BTreeMap<String, serde_yaml::Value>,
}

#[derive(Debug, Deserialize)]
struct Project {
    framework: String,
}

#[derive(Debug, Deserialize)]
struct Database {
    provider: String,
    url_env: String,
}

pub fn validate(path: &Path) -> Result<()> {
    let source = fs::read_to_string(path)
        .with_context(|| format!("Não foi possível ler {}", path.display()))?;
    let schema: Schema = serde_yaml::from_str(&source)
        .with_context(|| format!("YAML inválido em {}", path.display()))?;

    if schema.version != 1 {
        bail!("Versão de schema não suportada: {}", schema.version);
    }
    if !FRAMEWORKS.iter().any(|f| f.as_str() == schema.project.framework) {
        bail!("Framework não suportado: {}", schema.project.framework);
    }
    if !["mysql", "mariadb", "postgresql"].contains(&schema.database.provider.as_str()) {
        bail!("Database provider não suportado: {}", schema.database.provider);
    }
    if !valid_identifier(&schema.database.url_env) {
        bail!("database.url_env deve ser um nome válido de variável de ambiente");
    }
    for (name, definition) in &schema.models {
        if !valid_identifier(name) {
            bail!("Nome de model inválido: {name}");
        }
        if !definition.is_mapping() {
            bail!("Model {name} deve ser um objeto YAML");
        }
    }
    println!(
        "Schema válido: {} (framework {}, provider {}, {} models).",
        path.display(),
        schema.project.framework,
        schema.database.provider,
        schema.models.len()
    );
    println!("Nota: validação estrutural inicial; migrations e geração de models virão em versões futuras.");
    Ok(())
}

fn valid_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supports_expected_frameworks() {
        assert_eq!(FRAMEWORKS.len(), 7);
        assert_eq!(Framework::Qbcore.as_str(), "qbcore");
    }

    #[test]
    fn rejects_unsafe_identifiers() {
        assert!(!valid_identifier("users; DROP TABLE users"));
        assert!(!valid_identifier("1users"));
        assert!(valid_identifier("vrp_user_data"));
    }

    #[test]
    fn parses_scaffold() {
        let parsed: Schema = serde_yaml::from_str(
            "version: 1\nproject:\n  framework: esx\ndatabase:\n  provider: mysql\n  url_env: DATABASE_URL\nmodels: {}\n",
        )
        .unwrap();
        assert_eq!(parsed.project.framework, "esx");
        assert_eq!(parsed.models.len(), 0);
    }
}
