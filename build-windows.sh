BASEPATH=$(dirname $0)
export PKG_CONFIG_ALLOW_CROSS=1
export PKG_CONFIG_SYSROOT_DIR=${BASEPATH}/packaging/gtk/lib
export PKG_CONFIG_PATH=${BASEPATH}/packaging/gtk/lib/pkgconfig
export PATH=${PATH}:${BASEPATH}/packaging/gtk/bin
export LIB=${BASEPATH}/packaging/gtk/lib:${BASEPATH}/packaging/gtk/bin
cargo build --release --target x86_64-pc-windows-gnu


