# FiveORM

**Database tooling for FiveM — CLI em Rust, API Lua e integração sem SQL repetitivo.**

> **Estado do projeto: v0.1 (MVP experimental).** O CLI já implementa `init`, `validate` e `setup`. O runtime implementa operações CRUD básicas em **MySQL/MariaDB via oxmysql**. **Migrations, `db push`, `db pull`, PostgreSQL e Studio ainda não estão implementados.**

## Por que o FiveORM?

O FiveORM busca uma experiência semelhante à do Prisma/Drizzle em resources FiveM: comandos simples e consultas modeladas, sem precisar escrever SQL manualmente ou fazer `exports` em cada arquivo Lua. O núcleo CLI é compilado em Rust; a integração FiveM é um resource **server-side em Lua**. O FiveM não executa Rust como linguagem de script nativa.

O plano é oferecer integrações para **vRPex, vRP, ESX, Creative, QBCore e Qbox**. Nesta primeira versão, a seleção do framework é metadado no `schema.yml`. A API Lua usa tabelas reais do banco e **ainda não possui adaptadores de cache/regras de negócio específicos de cada framework**.

## Requisitos

- Rust/Cargo estável para **compilar/instalar** a CLI (depois de instalada, basta o binário `fiveorm`; Node/npm não são necessários).
- FiveM server (FXServer) com `fxmanifest.lua`.
- MySQL/MariaDB com o resource `oxmysql` para consultas em runtime.
- A CLI de setup deve ser executada com acesso de escrita à pasta `resources/`.

## Instalação da CLI (desenvolvimento)

```bash
git clone https://github.com/luizndev/fiveorm.git
cd fiveorm
cargo install --path .
fiveorm --help
```

> O repositório pode exigir autenticação GitHub enquanto for privado. Binários pré-compilados e instaladores serão distribuídos posteriormente.

## Instalação sem Cargo (binários do GitHub Actions)

A cada commit na `main`, o workflow **Rust CI** compila executáveis para Linux e Windows e disponibiliza os artefatos `fiveorm-linux-x86_64` e `fiveorm-windows-x86_64`.

1. Abra **Actions → Rust CI → execução concluída com sucesso → Artifacts**.
2. Baixe o ZIP do seu sistema operacional e extraia o executável.
3. Coloque `fiveorm` (Linux) ou `fiveorm.exe` (Windows) em uma pasta do `PATH`; no Linux, execute `chmod +x fiveorm`.
4. Execute `fiveorm --help`.

Isso **não exige Node.js, npm ou Rust na VPS**. Como o repositório é privado, os artefatos só estão acessíveis para contas autorizadas no GitHub.

## Primeiros passos

Entre na pasta de um resource FiveM existente, por exemplo `server/resources/[local]/garagem`:

```bash
# Se schema.yml não existir, pergunta qual framework utilizar:
fiveorm init

# Para CI ou sem terminal interativo:
fiveorm init --framework vrpex
fiveorm init --framework vrp
fiveorm init --framework esx
fiveorm init --framework creative
fiveorm init --framework qbcore
fiveorm init --framework qbox

# Confirma o YAML, sem conectar ao banco:
fiveorm validate

# Instala o runtime em resources/fiveorm e integra o fxmanifest.lua
fiveorm setup
```

O `init` **não sobrescreve** um `schema.yml` existente. A opção `--framework` aceita `vrpex`, `vrp`, `esx`, `creative`, `qbcore`, `qbox` e `standalone` (também aceita `qbone` como alias de `qbcore`).

### Exemplo de schema.yml gerado

```yaml
# FiveORM v0.1 — schema inicial (nenhuma tabela é criada automaticamente)
version: 1
project:
  framework: vrpex
database:
  provider: mysql
  url_env: DATABASE_URL
models: {}
```

**Importante:** nesta fase, `schema.yml` serve para inicialização e validação. O campo `url_env` é reservado para conexões gerenciadas pela CLI em versões futuras. A conexão usada pelo FiveM runtime é a que já está configurada no **oxmysql/server.cfg**. A opção `postgresql` pode ser reconhecida na validação, mas o runtime v0.1 **não possui driver PostgreSQL**.

Não criamos tabelas fictícias baseadas no nome do framework: versões de Creative, vRPex etc. diferem em schemas e estrutura de cache.

## Como funciona o setup

O comando:

1. Localiza a pasta `resources/` acima do resource atual (ou recebe `--resources-dir`).
2. Instala um resource independente em `resources/fiveorm`, copiando o loader Lua e os exports de banco.
3. Adiciona `server_script '@fiveorm/init.lua'` e `dependency 'fiveorm'` ao `fxmanifest.lua` do resource consumidor, **antes dos server scripts existentes**.
4. Caso encontre um `server.cfg` com `ensure oxmysql`, adiciona `ensure fiveorm` logo abaixo.
5. Cria arquivos `*.fiveorm.bak` antes de alterar manifests/configurações, não sobrescrevendo backups existentes.

O setup é idempotente para arquivos gerados sem alterações. Se detectar uma instalação modificada do runtime, **interrompe sem sobrescrever**.

Se rodar a CLI na raiz do servidor, sem um resource selecionado:

```bash
fiveorm setup --resources-dir ./resources --resource './resources/[local]/garagem'
```

Se o `server.cfg` estiver em outro local:

```bash
fiveorm setup --resources-dir ./resources \
  --resource './resources/[local]/garagem' \
  --server-cfg ./server.cfg
```

Se `ensure oxmysql` não existir literalmente no `server.cfg` (por exemplo, ao iniciar um grupo de resources), o FiveORM não tentará adivinhar a ordem. Configure manualmente **oxmysql → fiveorm → resource consumidor**.

### Manifest gerado (exemplo)

```lua
fx_version 'cerulean'
game 'gta5'

dependency 'fiveorm'
server_script '@fiveorm/init.lua'

server_scripts {
    'server/main.lua'
}
```

`db` passa a existir como global Lua **somente nos server scripts daquele resource**. Não existe injeção automática de globais em outros resources, nem conexão direta no client.

## Consultas sem SQL

Após o setup, dentro de qualquer **server script** listado no manifest integrado:

```lua
-- Ler linhas da tabela real vrp_users
local players = db.vrp_users.findMany({
    where = { banned = false },
    take = 20,
    skip = 0,
    orderBy = { id = 'desc' }
})

local user = db.vrp_users.findFirst({
    where = { id = 1001 }
})

-- INSERT retorna o insertId, não a linha completa:
local id = db.my_custom_table.create({
    data = { label = 'Exemplo' }
})

-- UPDATE / DELETE retornam quantidade de linhas afetadas
local updated = db.my_custom_table.update({
    where = { id = id },
    data = { label = 'Novo nome' }
})

local deleted = db.my_custom_table.delete({
    where = { id = id }
})
```

### Comportamento e segurança da v0.1

- Métodos: `findMany`, `findFirst`, `findUnique` (alias inicial de `findFirst`, **sem validar unicidade**), `create`, `update`, `delete`.
- `findMany` limita a consulta a **100 linhas por padrão**, com `take` entre 1 e 1000.
- `where` suporta igualdade (`AND`), e `orderBy` aceita `asc`/`desc`. Filtros avançados, joins, transações e relacionamentos não estão prontos.
- Nomes de tabelas/colunas são validados como identificadores SQL simples. Valores são enviados como **parâmetros** para o oxmysql, nunca concatenados ao SQL.
- `update` e `delete` recusam `where` vazio.
- O runtime não consulta `schema.yml` nem aplica alterações ao banco automaticamente.
- Consultas `await` exigem um contexto de execução FiveM compatível com coroutines; evite invocá-las em inicialização inadequada.
- **Não altere diretamente dinheiro, inventário ou estados de jogadores online se o framework os mantiver em memória.** Use suas APIs de negócio para evitar dados sobrescritos.

## Comandos disponíveis

| Comando | Implementação |
| --- | --- |
| `fiveorm init` | ✅ Interativo; gera `schema.yml` |
| `fiveorm init --framework esx` | ✅ Modo não interativo |
| `fiveorm validate` | ✅ Validação inicial de configuração |
| `fiveorm setup` | ✅ Instala runtime e integra resource |
| `fiveorm db pull` | 📋 Planejado: introspecção |
| `fiveorm db push` | 📋 Planejado: sincronização segura |
| `fiveorm migrate dev/deploy` | 📋 Planejado |
| `fiveorm studio` | 📋 Planejado |
| PostgreSQL runtime | 📋 Planejado |

## Organização

```text
fiveorm/
├── src/
│   ├── main.rs           # CLI clap
│   ├── schema.rs         # init e validate
│   └── setup.rs          # instalação segura FiveM
├── runtime/
│   └── fiveorm/
│       ├── fxmanifest.lua
│       ├── init.lua      # define db global no resource consumidor
│       └── server/main.lua  # CRUD por oxmysql
├── examples/garage/
├── .github/workflows/ci.yml
└── Cargo.toml
```

## Desenvolvimento

```bash
cargo test
cargo run -- init --framework creative
cargo run -- validate
cargo build --release
```

O GitHub Actions executa testes, build e um smoke test dos comandos de CLI. O runtime Lua ainda requer validação integrada em um FXServer real com oxmysql.

### Roadmap

1. Definições completas de models no YAML e validação de colunas e relacionamentos.
2. Introspecção de schemas reais (`db pull`) e geração de migrations versionadas.
3. Aplicação de migrations com diff e proteção contra operações destrutivas.
4. Autocomplete e tipagem LuaLS gerada a partir do schema.
5. Adapters para vRPex/Creative/ESX/QBCore/Qbox (sem contornar caches).
6. PostgreSQL runtime e migração guiada de bancos legados.
7. FiveORM Studio local com autenticação e acesso seguro.

**FiveORM é um projeto experimental.** Faça backup do banco e da configuração do servidor antes de testar em produção.
