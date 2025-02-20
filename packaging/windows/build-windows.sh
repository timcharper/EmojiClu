# you should run this script from the project root
BASEPATH=.

export PKG_CONFIG_ALLOW_CROSS=1
export PKG_CONFIG_SYSROOT_DIR=${BASEPATH}/packaging/windows/gtk/lib
export PKG_CONFIG_PATH=${BASEPATH}/packaging/windows/gtk/lib/pkgconfig
export PATH=${PATH}:${BASEPATH}/packaging/windows/gtk/bin
export LIB=${BASEPATH}/packaging/windows/gtk/lib:${BASEPATH}/packaging/windows/gtk/bin
cargo build --release --target x86_64-pc-windows-gnu


