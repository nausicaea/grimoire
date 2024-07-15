#!/usr/bin/env fish

argparse -N 1 -X 1 'p/proxy=' 'u/user-agent=' -- $argv; or return

set -q _flag_p; or set -l _flag_p "socks5h://172.16.1.2:9050"
set -q _flag_u; or set -l _flag_u "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36"
set -l PROXY "$_flag_p"
set -l UA "$_flag_u"
set -l FQDNS (path resolve "$argv[1]")

set -l WORKING_DIRECTORY (pwd)
set -l STATE_FILE (path normalize "$WORKING_DIRECTORY/http_recon_state")

# Switch to a temporary directory
set -l TEMP_FILE (mktemp)

# Build the cURL config
printf 'user-agent = "%s"\nproxy = "%s"\nproxytunnel\nparallel\nmax-time = 10\nhead\ninsecure\nsilent\nwrite-out = "%%output{>>%s}%%{response_code} %%{url}\\\\n"\n' $UA $PROXY $STATE_FILE > $TEMP_FILE
for fqdn in (shuf $FQDNS)
    printf 'url = http://%s\nurl = https://%s\n' $fqdn $fqdn >> $TEMP_FILE
end

# Run cURL on the list
exec curl --config $TEMP_FILE > /dev/null
