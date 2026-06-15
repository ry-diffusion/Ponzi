#!/bin/bash
set -e

echo "=== Instalando pingit-tablet-driver ==="

# Compilar
cargo build --release

# Copiar binário
sudo cp target/release/pingit-tablet-driver /usr/local/bin/
echo "Binário instalado em /usr/local/bin/pingit-tablet-driver"

# Criar diretório de config
sudo mkdir -p /etc/pingit-tablet
sudo cp config.toml /etc/pingit-tablet/
echo "Config copiada para /etc/pingit-tablet/config.toml"

# Instalar udev rule
sudo cp 99-pingit-tablet.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules
sudo udevadm trigger
echo "Regra udev instalada"

# Instalar serviço systemd
sudo cp pingit-tablet-driver.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable pingit-tablet-driver
echo "Serviço systemd habilitado"

echo ""
echo "=== Instalação completa! ==="
echo "Para iniciar agora: sudo systemctl start pingit-tablet-driver"
echo "Para ver logs: journalctl -u pingit-tablet-driver -f"
echo "Para editar config: sudo nano /etc/pingit-tablet/config.toml"
