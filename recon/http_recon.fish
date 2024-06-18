#!/usr/bin/env fish

argparse -N 1 -X 1 'p/proxy=' -- $argv; or return

set -q _flag_p; or set -l _flag_p "socks5h://172.16.1.2:9050"
set -l PROXY "$_flag_p"
set -l TARGETS "$argv[1]"
set -l STATE_FILE "state"

touch $STATE_FILE

for fqdn in (shuf "$TARGETS")
    set -l UA "$(printf '%8X' (random 0 4294967296) | sed 's/ /0/g')"

    if grep "$fqdn" $STATE_FILE > /dev/null
        printf '%s found in state file\n' $fqdn
        continue
    end

    set -l HTTP_STATUS (curl --user-agent "$UA" --proxy "$PROXY" -pLIs -o /dev/null -w '%{http_code}' "http://$fqdn")
    if test $status -eq 0
        printf '%s %s\n' $fqdn $HTTP_STATUS | tee -a $STATE_FILE
    end
end
