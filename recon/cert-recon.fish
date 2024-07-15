#!/usr/bin/env fish

argparse -N 1 -X 1 -- $argv; or return

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

exec psql -h crt.sh -U guest -w -d certwatch --csv -c $SQL_QUERY
