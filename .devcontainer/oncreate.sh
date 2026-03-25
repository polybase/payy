#!/usr/bin/env sh

sed -i 's/"runOn": "default",/"runOn": "folderOpen",/g' .vscode/tasks.json

avdmanager create avd -n MyDevice -k 'system-images;android-34;google_apis;x86_64' -d pixel

(
    cd /tmp &&\
    wget https://github.com/Genymobile/scrcpy/releases/download/v3.0/scrcpy-linux-v3.0.tar.gz &&\
    sudo tar -xvf scrcpy-linux-v3.0.tar.gz -C /var/lib &&\
    sudo ln -s /var/lib/scrcpy-linux-v3.0/scrcpy_bin /usr/bin/scrcpy &&\
    rm scrcpy-linux-v3.0.tar.gz
)

# Trigger rustup toolchain install.
# This should be installed by the devcontainer feature,
# but there seems to be a race condition that causes the tasks
# to download the toolchain, and if multiple try to download it at once,
# they fail and the installed toolchain is broken.
cargo --version