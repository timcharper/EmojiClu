BASEPATH=$(dirname $0)
export PKG_CONFIG_ALLOW_CROSS=1
export PKG_CONFIG_SYSROOT_DIR=${BASEPATH}/bundle/gtk/lib
export PKG_CONFIG_PATH=${BASEPATH}/bundle/gtk/lib/pkgconfig
export PATH=${PATH}:${BASEPATH}/bundle/gtk/bin
export LIB=${BASEPATH}/bundle/gtk/lib:${BASEPATH}/bundle/gtk/bin
cargo build --release --target x86_64-pc-windows-gnu


