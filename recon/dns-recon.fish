#!/usr/bin/env fish

argparse -N 1 -X 1 'd/dns-server=' -- $argv; or return

set -q _flag_d; or set -l _flag_d "172.16.1.2"
set -l DNS_SERVER "$_flag_d"
set -l FQDNS (path resolve "$argv[1]")

for fqdn in (shuf $FQDNS)
    nslookup -type=a $fqdn $DNS_SERVER \
        | awk 'BEGIN { RS = ""; FS = "\n" } /Non-authoritative answer:/ { print $2,$3 }' \
        | awk '{ print $2", "$4 }'
end
