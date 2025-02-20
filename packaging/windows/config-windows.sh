#!/bin/bash
set -x
BASEPATH="$(realpath $(dirname $0)/../../)"

if [ ! -d ${HOME}/.wine-affinity ]; then
  echo "wine-affinity environment not found"
  exit 1
fi

if [ ! -d ${BASEPATH}/packaging/windows/gtk ]; then
  echo "gtk not found"
  exit 1
fi

VULKAN_SDK_PATH=${HOME}/.wine-affinity/drive_c/VulkanSDK/1.4.304.0
GTK_LIB_PATH=${BASEPATH}/packaging/windows/gtk/lib
TARGET_PATH=${GTK_LIB_PATH}/usr/lib/x86_64-linux-gnu
mkdir -p ${GTK_LIB_PATH}/usr/lib

ln -sf ${GTK_LIB_PATH} ${GTK_LIB_PATH}/usr/lib/x86_64-linux-gnu

GOBJECT_PKG_CONF=$GTK_LIB_PATH/pkgconfig/gobject-2.0.pc

# Update line prefix=... to prefix=./packaging/windows/gtk
sed -i "s|^prefix=.*|prefix=${GTK_LIB_PATH}|" $GOBJECT_PKG_CONF

# symlink in the vulkan deps

ln -sf ${VULKAN_SDK_PATH}/Lib/vulkan-1.lib ${GTK_LIB_PATH}/vulkan.lib
#ln -sf ${VULKAN_SDK_PATH}/Lib/vulkan-1.lib ${GTK_LIB_PATH}/libvulkan.lib.a # 
cp /home/tim/.wine-affinity/drive_c/windows/system32/vulkan-1.dll ${GTK_LIB_PATH}/libvulcan.dll.a
