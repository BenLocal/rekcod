#!/bin/sh
set -e

SYSTEMD_PATH="/etc/systemd/system"
FILE_REKCOD_SERVICE="${SYSTEMD_PATH}/rekcod.service"
FILE_REKCOD_ENV="/etc/rekcod/rekcod.env"

quote_indent() {
    printf ' \\\n'
    for arg in "$@"; do
        printf '\t%s \\\n' "$(quote "$arg")"
    done
}

setup_env() {
    CMD_REKCOD=$1
    CMD_REKCOD_EXEC="${CMD_REKCOD}$(quote_indent "$@")"
}

create_env() {
    touch ${FILE_REKCOD_ENV}
    chmod 0600 ${FILE_REKCOD_ENV}
    sh -c export | while read x v; do echo $v; done | grep -E '^(REKCOD|DOCKER)_' | tee ${FILE_REKCOD_ENV} >/dev/null
}

create_systemd_service_file() {
    if [ ! -f "$FILE_REKCOD_SERVICE" ]; then
        mkdir -p ${SYSTEMD_PATH}
        cat >${FILE_REKCOD_SERVICE} <<EOF
[Unit]
Description=Rekcodd

[Service]
Type=simple
EnvironmentFile=-${FILE_REKCOD_ENV}
Restart=always
RestartSec=1
ExecStart=/usr/bin/rekcodd ${REKCOD_EXEC}
StandardOutput=journal

[Install]
WantedBy=multi-user.target
EOF
    fi
}

start_and_enable_service() {
    systemctl daemon-reload
    systemctl enable rekcod
    systemctl start rekcod
}

stop_service() {
    systemctl stop rekcod
}

setup_env "$@"
create_env
create_systemd_service
start_and_enable_service