#!/bin/sh
set -e

SYSTEMD_PATH="/etc/systemd/system"
FILE_REKCOD_SERVICE="${SYSTEMD_PATH}/rekcod.service"
FILE_REKCOD_ENV="/etc/rekcod/rekcod.env"

quote() {
    for arg in "$@"; do
        printf '%s\n' "$arg" | sed "s/'/'\\\\''/g;1s/^/'/;\$s/\$/'/"
    done
}

quote_indent() {
    printf ' \\\n'
    for arg in "$@"; do
        printf '\t%s \\\n' "$(quote "$arg")"
    done
}

setup_env() {
    CMD_REKCOD_EXEC="$(quote_indent "$@")"
    echo "CMD_REKCOD_EXEC=${CMD_REKCOD_EXEC}"
}

create_env() {
    touch ${FILE_REKCOD_ENV}
    chmod 0600 ${FILE_REKCOD_ENV}
    sh -c export | while read x v; do echo $v; done | grep -E '^(REKCOD|DOCKER)_' | tee ${FILE_REKCOD_ENV} >/dev/null
}

create_systemd_service_file() {
    mkdir -p ${SYSTEMD_PATH}
    cat >${FILE_REKCOD_SERVICE} <<EOF
[Unit]
Description=Rekcodd

[Service]
Type=simple
EnvironmentFile=-${FILE_REKCOD_ENV}
Restart=always
RestartSec=1
ExecStart=/usr/bin/rekcodd ${CMD_REKCOD_EXEC}

StandardOutput=journal

[Install]
WantedBy=multi-user.target
EOF
}

start_and_enable_service() {
    systemctl daemon-reload
    systemctl enable rekcod
    systemctl start rekcod
}

try_stop_service() {
    # systemctl status rekcod
    systemctl stop rekcod
}

offline_install() {
    cp -rf ./rekcod /usr/bin/rekcod
    cp -rf ./rekcodd /usr/bin/rekcodd
}

echo "try_stop_service"
try_stop_service
echo "offline_install"
offline_install
echo "setup_env"
setup_env "$@"
echo "create_env"
create_env
echo "create_systemd_service_file"
create_systemd_service_file
echo "start_and_enable_service"
start_and_enable_service