#!/usr/bin/env fish

argparse -N 1 -X 1 'd/dns-server=' -- $argv; or return

set -q _flag_d; or set -l _flag_d "172.16.1.2"
set -l DNS_SERVER "$_flag_d"
string match -q -g --regex \
    '^(?=.{1,253})(?!.*--.*)(?P<FQDN>(?:(?!-)(?![0-9])[a-zA-Z0-9-]{1,63}(?<!-)\.){1,}(?:(?!-)[a-zA-Z0-9-]{1,63}(?<!-)))$' "$argv[1]"; or return

set -l SQL_QUERY "\
SELECT DISTINCT cai.NAME_VALUE
FROM certificate_and_identities AS cai
WHERE
    plainto_tsquery('certwatch', '$FQDN') @@ identities(cai.certificate)
    AND (cai.NAME_TYPE = '2.5.4.3' OR cai.NAME_TYPE LIKE 'san:%')
    AND cai.NAME_VALUE LIKE '%.$FQDN'
    --AND coalesce(x509_notAfter(cai.CERTIFICATE), 'infinity'::timestamp) >= date_trunc('year', now() AT TIME ZONE 'UTC')
    --AND x509_notAfter(cai.CERTIFICATE) >= now() AT TIME ZONE 'UTC' \
"

set -l CRT_SH_IP (nslookup -type=a 'crt.sh' | grep -o '[0-9]\+\.[0-9]\+\.[0-9]\+\.[0-9]\+' | grep -v '127.[0-9]\+.[0-9]\+.[0-9]\+')

exec psql -h CRT_SH_IP -U guest -w -d certwatch --csv -c $SQL_QUERY
