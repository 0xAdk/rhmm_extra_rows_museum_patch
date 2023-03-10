#!/usr/bin/env bash

FTP_USER="${FTP_USER:-user}"
FTP_HOST="${FTP_HOST:-}"
FTP_PORT="${FTP_PORT:-5000}"

if [ -z "$FTP_HOST" ]; then
	# shellcheck disable=2016
	printf '$FTP_HOST not set\n'
	printf 'set it to the ip address of your 3ds\n'
	printf 'example:\n'
	printf '\texport FTP_HOST=192.168.0.51\n'
	printf '\t%s\n' "$0"
	exit 1
fi

ftp -inv "$FTP_HOST" "$FTP_PORT" <<-EOF
	user $FTP_USER
	lcd output
	cd /luma/titles/000400000018A400
	put code.ips
	bye
EOF
