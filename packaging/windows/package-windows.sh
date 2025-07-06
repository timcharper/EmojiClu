#!/bin/bash
set -x
BASEPATH="$(realpath $(dirname $0)/../../)"
PACKAGE_PATH=${BASEPATH}/packaging/windows
GTK_PATH=${PACKAGE_PATH}/gtk
GTK_LIB_PATH=${GTK_PATH}/lib

# rm -rf ${BASEPATH}/bundle/emojiclu
mkdir -p ${PACKAGE_PATH}/emojiclu/bin
mkdir -p ${PACKAGE_PATH}/emojiclu/share/themes
mkdir -p ${PACKAGE_PATH}/emojiclu/share/icons

cp ./target/x86_64-pc-windows-gnu/release/emojiclu.exe ${PACKAGE_PATH}/emojiclu/bin/emojiclu.exe
for f in ${GTK_LIB}/bin/*.dll; do
  cp $f ${PACKAGE_PATH}/emojiclu/bin/
done

mkdir -p ${PACKAGE_PATH}/emojiclu/lib/gdk-pixbuf-2.0/2.10.0/loaders
cp ${GTK_LIB_PATH}/gdk-pixbuf-2.0/2.10.0/loaders/pixbufloader_svg.dll ${PACKAGE_PATH}/emojiclu/lib/gdk-pixbuf-2.0/2.10.0/loaders/
cp ${GTK_LIB_PATH}/gdk-pixbuf-2.0/2.10.0/loaders/loaders.cache ${PACKAGE_PATH}/emojiclu/lib/gdk-pixbuf-2.0/2.10.0/loaders/
rsync -av ${GTK_PATH}/share/ ${PACKAGE_PATH}/emojiclu/share/

# convert icons
rm -rf ${PACKAGE_PATH}/emojiclu/share/icons/Adwaita/48x48
rm -rf ${PACKAGE_PATH}/emojiclu/share/icons/Adwaita/symbolic
mkdir -p ${PACKAGE_PATH}/emojiclu/share/icons/Adwaita/48x48/actions
mkdir -p ${BASEPATH}/target/icons
for icon in edit-undo-symbolic edit-redo-symbolic; do
    inkscape -z --export-filename ${BASEPATH}/target/icons/${icon}.png -w 48 -h 48 ${GTK_PATH}/share/icons/Adwaita/symbolic/actions/${icon}.svg
done
cp -r ${BASEPATH}/target/icons/* ${PACKAGE_PATH}/emojiclu/share/icons/Adwaita/48x48/actions/

