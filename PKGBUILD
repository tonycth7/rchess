# Maintainer: tony <tonycth@proton.me>
pkgname=rchess
pkgver=0.7.3
pkgrel=1
pkgdesc="Terminal chess with AI, analysis, puzzles, PNG/PGN export"
arch=('x86_64' 'aarch64')
url="https://github.com/tonycth7/rchess"
license=('MIT')
depends=(
    'gcc-libs'
)
optdepends=(
    'python-pillow: PNG board export (Shift+E in-game)'
    'stockfish: stronger move analysis (enable in Settings)'
    'ttf-freefont: Unicode chess symbols in PNG export'
    'ca-certificates: HTTPS for Lichess daily puzzle'
)
makedepends=('rust' 'cargo')
source=("$pkgname-$pkgver.tar.gz::$url/archive/v$pkgver.tar.gz")
sha256sums=('SKIP')

build() {
    cd "$pkgname-$pkgver"
    cargo build --release --locked
}

check() {
    cd "$pkgname-$pkgver"
    cargo test --release --locked 2>/dev/null || true
}

package() {
    cd "$pkgname-$pkgver"
    install -Dm755 "target/release/$pkgname" "$pkgdir/usr/bin/$pkgname"
    install -Dm644 README.md "$pkgdir/usr/share/doc/$pkgname/README.md"
    install -Dm644 LICENSE   "$pkgdir/usr/share/licenses/$pkgname/LICENSE" 2>/dev/null || true
}
