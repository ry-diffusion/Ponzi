pkgname=ponzi
pkgver=0.1.0
pkgrel=1
pkgdesc='Linux userspace driver & configurator for Gotop/SZ PING-IT T1161 drawing tablets'
arch=('x86_64')
url='https://github.com/ry-diffusion/Ponzi'
license=('MIT')
depends=('libusb' 'systemd-libs')
makedepends=('cargo' 'pkg-config')
source=("$pkgname-$pkgver.tar.gz::$url/archive/refs/heads/develop.tar.gz")
sha256sums=('SKIP')

prepare() {
    cd "Ponzi-develop"
    export RUSTUP_TOOLCHAIN=stable
    cargo fetch --locked --target "$(rustc -vV | sed -n 's/host: //p')"
}

build() {
    cd "Ponzi-develop"
    export RUSTUP_TOOLCHAIN=stable
    export CARGO_TARGET_DIR=target
    cargo build --release -p ponzi-driver
    cargo build --release -p ponzi-ui
}

package() {
    cd "Ponzi-develop"

    install -Dm755 target/release/ponzi-driver "$pkgdir/usr/bin/ponzi-driver"
    install -Dm755 target/release/ponzi-ui "$pkgdir/usr/bin/ponzi-ui"

    install -Dm644 99-pingit-tablet.rules "$pkgdir/etc/udev/rules.d/99-ponzi-tablet.rules"
    install -Dm644 ponzi-driver.service "$pkgdir/usr/lib/systemd/system/ponzi-driver.service"
    install -Dm644 ponzi-driver/config.toml "$pkgdir/etc/ponzi/config.toml"
    install -Dm644 ponzi.desktop "$pkgdir/usr/share/applications/ponzi.desktop"
}
