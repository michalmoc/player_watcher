# Maintainer: Michał Moczulski
pkgname=player_watcher
pkgver=0.1.0.r0.gd7fdcd3
pkgrel=1
pkgdesc="Track which media player is currently active via MPRIS."
url="https://github.com/michalmoc/player_watcher"
license=("MIT")
arch=("x86_64")
makedepends=("git" "cargo")
source=('player_watcher::git+https://github.com/michalmoc/player_watcher.git')
sha256sums=("SKIP")

pkgver() {
    cd "$srcdir/player_watcher"
    (git describe --long --tags || echo "$pkgver") | sed 's/^v//;s/\([^-]*-g\)/r\1/;s/-/./g'
}

build() {
    return 0
}

package() {
    cd "$srcdir/player_watcher"
    usrdir="$pkgdir/usr"
    mkdir -p $usrdir
    cargo install --no-track --path . --root "$usrdir"
}

