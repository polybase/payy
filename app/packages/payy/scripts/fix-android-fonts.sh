#!/bin/bash

# create the xml font direcory
ANDROID_FONTS_DIR=./android/app/src/main/res/font
echo "Copying fonts to Android xml fonts directory..."
mkdir -p ${ANDROID_FONTS_DIR}
cp ./assets/fonts/*.otf ${ANDROID_FONTS_DIR}

# rename the fonts as per Android's assets naming requirements
# Adapted from: https://github.com/jsamr/react-native-font-demo?tab=readme-ov-file#1-copy-and-rename-assets-to-the-resource-font-folder
echo "Fixing fonts names to conform to Android's assets naming requirements..."
if [[ -d "${ANDROID_FONTS_DIR}" && ! -z "${ANDROID_FONTS_DIR}" ]]; then
  pushd "${ANDROID_FONTS_DIR}";
  for file in *.otf; do
    typeset normalized="${file//-/_}";
    normalized=$(echo "$normalized" | tr '[:upper:]' '[:lower:]')
    mv "$file" "$normalized"
  done
  popd
fi

# copy steradian.xml to the xml fonts directory
FONTS_XML_FILE="./assets/fonts/android/steradian.xml"
echo "Copying ${FONTS_XML_FILE} to the Android xml fonts directory..."
cp ${FONTS_XML_FILE} ${ANDROID_FONTS_DIR}
