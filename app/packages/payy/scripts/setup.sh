#!/usr/bin/env bash

set -euxo pipefail

# Determines if we are building using `yarn` or `eas` cli
EAS_BUILD=${EAS_BUILD:-""}
APP_VARIANT=${APP_VARIANT:-""}

echo "EAS_BUILD = ${EAS_BUILD}"
echo "APP_VARIANT = ${APP_VARIANT}"
echo "DEV_BUILD = ${DEV_BUILD:-}"

ANDROID_PREBUILD_DIR="android"
IOS_PREBUILD_DIR="ios"

# for the `try_yarn_install function`
echo "Sourcing retry_yarn.sh"
source "$(dirname "$0")/retry_yarn.sh"

if [[ -z "$EAS_BUILD" ]]; then
    # Clean up `app/node_modules`, `payy/ios`, and `payy/android`
    # as part of build
    if [[ "$@" == *"--clean"* || "$@" == *"--full-clean"* ]]
    then
        (
            echo "Cleaning up app artifacts"
            set +e
            set -x
            echo "Cleaning app/node_modules..."
            rm -rf ../../node_modules
            echo "Cleaning payy/ios and payy/android directories..."
            rm -rf ios android
            set -e
        )
    fi

# Trigger yarn install for `expo` to be available for prebuild
try_yarn_install
fi

# prepare the android backup config plugin
echo "Preparing the Android Key-Value Backup Agent plugin..."
yarn workspace android-kv-backup-agent clean
yarn workspace android-kv-backup-agent prepare

# prepare the ios userdefaults (suite) config plugin
echo "Preparing the UserDefaults native module..."
yarn workspace user-defaults-suite-ios clean
yarn workspace user-defaults-suite-ios prepare

# prepare the call detection native module
echo "Preparing the Call Detection native module..."
yarn workspace call-detection clean
yarn workspace call-detection prepare

# Function to check if cargo is installed
function check_cargo() {
    command -v cargo >/dev/null 2>&1
}

# Check if cargo is installed. If not, install it.
if ! check_cargo; then
    echo "Cargo is not installed. Installing now..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source $HOME/.cargo/env
else
    echo "Cargo is already installed."
fi

if [[ -z "$EAS_BUILD" ]]; then
    # run `prebuild` iff the `android` and `iOS` directories do not exist
    if [[ ! -d ${ANDROID_PREBUILD_DIR} ]] || [[ ! -d ${IOS_PREBUILD_DIR} ]]
    then
        echo "Missing android and/or iOS directories, running expo prebuild..."
        npx expo prebuild
    fi
fi

# link the font assets (only for `iOS`)
npx -y react-native-asset -y --ios-assets ./assets/fonts

# link the font assets (only for `Android`)
bash scripts/fix-android-fonts.sh

# Don't build it again during eas build
if [[ -z "$EAS_BUILD" ]]; then
    if [[ "$APP_VARIANT" == "storybook" ]]; then
        echo "Storybook variant detected; skipping react-native-rust-bridge build."
    else
        # run setup for the react-native-rust-bridge dependency
        yarn workspace react-native-rust-bridge clean
        yarn workspace react-native-rust-bridge setup
        yarn workspace react-native-rust-bridge build "$@" # pass any flags such as `--clean` to the build script
        yarn workspace react-native-rust-bridge prepare
    fi
fi
