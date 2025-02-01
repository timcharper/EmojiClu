#!/bin/bash
set -x
BASEPATH="$(realpath $(dirname $0))"
GTK_LIB_PATH=${BASEPATH}/bundle/gtk/lib

# rm -rf ${BASEPATH}/bundle/gnomeclu
mkdir -p ${BASEPATH}/bundle/gnomeclu/bin
mkdir -p ${BASEPATH}/bundle/gnomeclu/share/themes
mkdir -p ${BASEPATH}/bundle/gnomeclu/share/icons

cp ./target/x86_64-pc-windows-gnu/release/gnomeclu.exe ${BASEPATH}/bundle/gnomeclu/bin/gnomeclu.exe
for f in ${BASEPATH}/bundle/gtk/bin/*.dll; do
  cp $f ${BASEPATH}/bundle/gnomeclu/bin/
done

mkdir -p ${BASEPATH}/bundle/gnomeclu/lib/gdk-pixbuf-2.0/2.10.0/loaders
cp ${BASEPATH}/bundle/gtk/lib/gdk-pixbuf-2.0/2.10.0/loaders/pixbufloader_svg.dll ${BASEPATH}/bundle/gnomeclu/lib/gdk-pixbuf-2.0/2.10.0/loaders/
cp ${BASEPATH}/bundle/gtk/lib/gdk-pixbuf-2.0/2.10.0/loaders/loaders.cache ${BASEPATH}/bundle/gnomeclu/lib/gdk-pixbuf-2.0/2.10.0/loaders/
rsync -av ./bundle/gtk/share/ ${BASEPATH}/bundle/gnomeclu/share/

# convert icons
rm -rf ${BASEPATH}/bundle/gnomeclu/share/icons/Adwaita/48x48
rm -rf ${BASEPATH}/bundle/gnomeclu/share/icons/Adwaita/symbolic
mkdir -p ${BASEPATH}/bundle/gnomeclu/share/icons/Adwaita/48x48/actions
mkdir -p ${BASEPATH}/target/icons
for icon in edit-undo-symbolic edit-redo-symbolic; do
    inkscape -z --export-filename ${BASEPATH}/target/icons/${icon}.png -w 48 -h 48 ${BASEPATH}/bundle/gtk/share/icons/Adwaita/symbolic/actions/${icon}.svg
done
cp -r ${BASEPATH}/target/icons/* ${BASEPATH}/bundle/gnomeclu/share/icons/Adwaita/48x48/actions/

