#!/bin/sh
# FiveORM installer for Linux x86_64. No Rust, Cargo, Node.js or npm required.
set -eu

repo="luizndev/fiveorm"
asset="fiveorm-linux-x86_64.tar.gz"
base="https://github.com/$repo/releases/latest/download"

fail() {
  printf 'FiveORM: %s\n' "$*" >&2
  exit 1
}

[ "$(uname -s)" = "Linux" ] || fail "Este instalador suporta Linux. No Windows, use install.ps1."
case "$(uname -m)" in
  x86_64|amd64) ;;
  *) fail "Arquitetura não suportada: $(uname -m). Disponível apenas para Linux x86_64." ;;
esac

command -v tar >/dev/null 2>&1 || fail "O comando tar é necessário."
if command -v curl >/dev/null 2>&1; then
  download() { curl --fail --location --silent --show-error --retry 3 "$1" --output "$2"; }
elif command -v wget >/dev/null 2>&1; then
  download() { wget --quiet --output-document="$2" "$1"; }
else
  fail "Instale curl ou wget para continuar."
fi

if command -v sha256sum >/dev/null 2>&1; then
  file_hash() { sha256sum "$1" | awk '{print $1}'; }
elif command -v shasum >/dev/null 2>&1; then
  file_hash() { shasum -a 256 "$1" | awk '{print $1}'; }
else
  fail "É necessário sha256sum ou shasum para verificar a integridade do arquivo."
fi

temp_dir="$(mktemp -d)" || fail "Não foi possível criar uma pasta temporária."
trap 'rm -rf "$temp_dir"' EXIT HUP INT TERM

printf 'FiveORM: baixando última versão para Linux x86_64...\n'
if ! download "$base/$asset" "$temp_dir/$asset"; then
  fail "Falha no download. Publique uma GitHub Release e torne o repositório público para instalação anônima."
fi
if ! download "$base/SHA256SUMS" "$temp_dir/SHA256SUMS"; then
  fail "Não encontrei SHA256SUMS na release. Instalação cancelada."
fi

expected="$(awk -v file="$asset" '$2 == file {print $1}' "$temp_dir/SHA256SUMS")"
[ -n "$expected" ] || fail "SHA256 não encontrado para $asset."
actual="$(file_hash "$temp_dir/$asset")"
[ "$expected" = "$actual" ] || fail "Verificação SHA256 falhou. Arquivo não será instalado."

tar -xzf "$temp_dir/$asset" -C "$temp_dir" || fail "Não foi possível extrair o executável."
[ -f "$temp_dir/fiveorm" ] || fail "A release não contém o executável fiveorm."

install_dir="$HOME/.local/bin"
mkdir -p "$install_dir" || fail "Não foi possível criar $install_dir."
cp "$temp_dir/fiveorm" "$install_dir/fiveorm" || fail "Não foi possível instalar o executável."
chmod 755 "$install_dir/fiveorm"
printf 'FiveORM instalado em: %s/fiveorm\n' "$install_dir"

path_line='export PATH="$HOME/.local/bin:$PATH"'
for profile in "$HOME/.profile" "$HOME/.bashrc"; do
  if [ ! -f "$profile" ] || ! grep -Fqx "$path_line" "$profile"; then
    printf '\n# FiveORM CLI\n%s\n' "$path_line" >> "$profile" || fail "Não foi possível atualizar $profile."
  fi
done
if [ "$(basename "$SHELL" 2>/dev/null)" = "zsh" ]; then
  profile="$HOME/.zshrc"
  if [ ! -f "$profile" ] || ! grep -Fqx "$path_line" "$profile"; then
    printf '\n# FiveORM CLI\n%s\n' "$path_line" >> "$profile" || fail "Não foi possível atualizar $profile."
  fi
fi

printf '\nVersão instalada:\n'
"$install_dir/fiveorm" --version
printf '\nAbra um novo terminal ou execute: export PATH="$HOME/.local/bin:$PATH"\n'
printf 'Depois use: fiveorm init\n'
